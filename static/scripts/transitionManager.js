(function () {
  class TransitionManager {
    /**
     * @param {Object} options
     * @param {HTMLElement} [options.overlay]
     * @param {string} [options.overlaySelector]
     * @param {number} [options.duration=600] - duration of each animation leg
     * @param {number} [options.bufferTime=0]   - pause at full cover (B) before continuing
     * @param {("slide"|"fade")} [options.transitionEffect="slide"]
     * @param {string} [options.easing='cubic-bezier(0.22, 1, 0.36, 1)']
     * @param {number} [options.zIndex=2147483647]
     * @param {boolean} [options.respectReducedMotion=true]
     * @param {string} [options.sessionKey='__tm_pending_out__']
     */
    constructor({
      overlay,
      overlaySelector,
      duration = 600,
      bufferTime = 0,
      transitionEffect = "slide",
      easing = 'cubic-bezier(0.22, 1, 0.36, 1)',
      zIndex = 2147483647,
      respectReducedMotion = true,
      sessionKey = '__tm_pending_out__'
    } = {}) {
      this.overlay = overlay || document.querySelector(overlaySelector || '');
      if (!this.overlay) {
        throw new Error('TransitionManager: overlay element not found. Provide overlay or overlaySelector.');
      }
      this.duration = duration;
      this.bufferTime = bufferTime;
      this.transitionEffect = transitionEffect === 'fade' ? 'fade' : 'slide';
      this.easing = easing;
      this.zIndex = zIndex;
      this.sessionKey = sessionKey;
      this.respectReducedMotion = respectReducedMotion;

      this._isAnimating = false;

      this._setupOverlayStyles();
      this._installResizeHandler();

      // On page load, animate B → C (with optional buffer), then park at A.
      queueMicrotask(() => this._animateOutThenHide());
    }

    /** Public: navigate with transition (A→B, optional buffer, then navigate) */
    async transitionTo(url) {
      if (this._isAnimating) return false;

      if (this._shouldReduceMotion()) {
        window.location.assign(url);
        return false;
      }

      this._isAnimating = true;
      try {
        await this._animateInToCover(); // A → B
        if (this.bufferTime > 0) {
          await this._wait(this.bufferTime);
        }
        // Mark in case you later decide to gate B→C on load
        sessionStorage.setItem(this.sessionKey, '1');
        window.location.assign(url);
      } finally {
        this._isAnimating = false;
      }
      return false;
    }

    /** Public helper for inline onclick on anchors. */
    handleLinkClick(e, anchor) {
      if (!anchor || !anchor.href) return true;
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || anchor.target === '_blank') return true;
      e.preventDefault();
      return this.transitionTo(anchor.href) === false ? false : true;
    }

    // ---------- Internals ----------

    _setupOverlayStyles() {
      const el = this.overlay;
      const base = {
        position: 'fixed',
        inset: '0',
        width: '100vw',
        height: '100vh',
        zIndex: String(this.zIndex),
        pointerEvents: 'none'
      };
      for (const [k, v] of Object.entries(base)) {
        el.style.setProperty(k, v, 'important');
      }

      // Prepare effect-specific properties without overriding your inline boot state.
      if (this.transitionEffect === 'slide') {
        el.style.setProperty('will-change', 'transform', 'important');
        // do not force transform here; HTML may start at B (translateX(0%)) to cover on load
      } else { // fade
        el.style.setProperty('will-change', 'opacity', 'important');
        // If author didn't set an initial opacity, assume we're covering on load.
        if (!el.style.opacity) {
          el.style.opacity = '1';
        }
        // Ensure visibility is visible at start (author can override in HTML)
        if (!el.style.visibility) {
          el.style.visibility = 'visible';
        }
      }
    }

    _installResizeHandler() {
      this._onResize = () => {
        this.overlay.style.setProperty('width', '100vw', 'important');
        this.overlay.style.setProperty('height', '100vh', 'important');
      };
      window.addEventListener('resize', this._onResize, { passive: true });
      this._onResize();
    }

    _shouldReduceMotion() {
      return this.respectReducedMotion &&
        window.matchMedia &&
        window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    }

    _parkAtAAndHide() {
      const el = this.overlay;
      if (this.transitionEffect === 'slide') {
        el.style.visibility = 'hidden';
        el.style.transform = 'translateX(-100%)'; // A
      } else {
        el.style.opacity = '0'; // A (transparent)
        el.style.visibility = 'hidden';
      }
    }

    _showAtB() {
      const el = this.overlay;
      el.style.visibility = 'visible';
      if (this.transitionEffect === 'slide') {
        el.style.transform = 'translateX(0%)'; // B
      } else {
        el.style.opacity = '1'; // B
      }
    }

    _hideAtC() {
      const el = this.overlay;
      if (this.transitionEffect === 'slide') {
        el.style.transform = 'translateX(100%)'; // C
        el.style.visibility = 'hidden';
      } else {
        el.style.opacity = '0'; // C (same as A visually)
        el.style.visibility = 'hidden';
      }
    }

    _animate(element, keyframes, options) {
      return new Promise((resolve, reject) => {
        const anim = element.animate(keyframes, options);
        anim.addEventListener('finish', () => resolve());
        anim.addEventListener('cancel', () => reject(new Error('animation canceled')));
      });
    }

    _wait(ms) {
      return new Promise(res => setTimeout(res, ms));
    }

    async _animateInToCover() {
      const el = this.overlay;
      el.style.visibility = 'visible';

      if (this.transitionEffect === 'slide') {
        // Start from A (left)
        el.style.transform = 'translateX(-100%)';
        await this._animate(
          el,
          [
            { transform: 'translateX(-100%)' }, // A
            { transform: 'translateX(0%)' }     // B
          ],
          { duration: this.duration, easing: this.easing, fill: 'forwards' }
        );
      } else {
        // Fade: A (opacity 0) -> B (opacity 1)
        el.style.opacity = '0';
        await this._animate(
          el,
          [
            { opacity: 0 }, // A
            { opacity: 1 }  // B
          ],
          { duration: this.duration, easing: this.easing, fill: 'forwards' }
        );
      }
    }

    async _animateOutThenHide() {
      if (this._shouldReduceMotion()) {
        this._hideAtC();
        requestAnimationFrame(() => this._parkAtAAndHide());
        return;
      }

      const el = this.overlay;
      // Ensure we're at B first
      this._showAtB();

      if (this.bufferTime > 0) {
        await this._wait(this.bufferTime);
      }

      try {
        if (this.transitionEffect === 'slide') {
          // B -> C (right)
          await this._animate(
            el,
            [
              { transform: 'translateX(0%)' },    // B
              { transform: 'translateX(100%)' }   // C
            ],
            { duration: this.duration, easing: this.easing, fill: 'forwards' }
          );
        } else {
          // Fade: B (1) -> C (0)
          await this._animate(
            el,
            [
              { opacity: 1 }, // B
              { opacity: 0 }  // C
            ],
            { duration: this.duration, easing: this.easing, fill: 'forwards' }
          );
        }
      } catch (_) {
        // ignore
      } finally {
        this._hideAtC();
        requestAnimationFrame(() => this._parkAtAAndHide());
      }
    }
  }

  window.TransitionManager = TransitionManager;
})();
