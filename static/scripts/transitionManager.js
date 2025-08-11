
(function () {
  class TransitionManager {
    /**
     * @param {Object} options
     * @param {HTMLElement} [options.overlay]
     * @param {string} [options.overlaySelector]
     * @param {number} [options.duration=600]
     * @param {number} [options.bufferTime=0]
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
      if (!this.overlay) throw new Error('TransitionManager: overlay element not found. Provide overlay or overlaySelector.');
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
      this._installLifecycleHandlers();

      // On classic loads, play B→C (then park at A).
      // On bfcache restores, pageshow handler (below) will handle it.
      queueMicrotask(() => this._animateOutThenHide());
    }

    /** Navigate with transition (A→B, optional buffer, then navigate) */
    async transitionTo(url) {
      if (this._isAnimating) return false;

      if (this._shouldReduceMotion()) {
        window.location.assign(url);
        return false;
      }

      this._isAnimating = true;
      try {
        await this._animateInToCover(); // A → B
        if (this.bufferTime > 0) await this._wait(this.bufferTime);
        // Mark that the *next* page should run B→C on show (helps reloads & some restores).
        sessionStorage.setItem(this.sessionKey, '1');
        window.location.assign(url);
      } finally {
        this._isAnimating = false;
      }
      return false;
    }

    /** Helper for inline onclick on anchors */
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

      if (this.transitionEffect === 'slide') {
        el.style.setProperty('will-change', 'transform', 'important');
        // Author can start with B coverage in HTML; we won't override here.
      } else {
        el.style.setProperty('will-change', 'opacity', 'important');
        if (!el.style.opacity) el.style.opacity = '1';
        if (!el.style.visibility) el.style.visibility = 'visible';
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

    _installLifecycleHandlers() {
      // When the page is shown (including bfcache restore), ensure B→C runs if needed.
      window.addEventListener('pageshow', (e) => {
        const pending = sessionStorage.getItem(this.sessionKey) === '1';
        // If restored from bfcache or we flagged a pending-out, run the hide sequence.
        if (e.persisted || pending) {
          this._animateOutThenHide().finally(() => {
            sessionStorage.removeItem(this.sessionKey);
          });
        } else {
          // On normal shows, make sure we're safely parked.
          this._parkAtAAndHide();
        }
      });

      // If the page is being hidden (bfcache candidate), cancel any running animations to avoid stuck states.
      window.addEventListener('pagehide', () => {
        try { this.overlay.getAnimations().forEach(a => a.cancel()); } catch (_) {}
      });

      // History traversal (some browsers fire popstate on restore). Belt-and-suspenders:
      window.addEventListener('popstate', () => {
        // If the overlay is visible (e.g., at B), ensure we clear it quickly.
        requestAnimationFrame(() => this._parkAtAAndHide());
      });

      // As an extra guard, if the tab becomes visible again mid-animation, ensure we finish hiding.
      document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible') {
          // If we ever got stuck half-way, finish the hide.
          this._hideAtC();
          requestAnimationFrame(() => this._parkAtAAndHide());
        }
      });
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
        el.style.opacity = '0'; // A
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
        el.style.opacity = '0';
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

    _wait(ms) { return new Promise(res => setTimeout(res, ms)); }

    async _animateInToCover() {
      const el = this.overlay;
      el.style.visibility = 'visible';

      if (this.transitionEffect === 'slide') {
        el.style.transform = 'translateX(-100%)';
        await this._animate(
          el,
          [{ transform: 'translateX(-100%)' }, { transform: 'translateX(0%)' }],
          { duration: this.duration, easing: this.easing, fill: 'forwards' }
        );
      } else {
        el.style.opacity = '0';
        await this._animate(
          el,
          [{ opacity: 0 }, { opacity: 1 }],
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

      // Ensure we're at B first
      this._showAtB();

      if (this.bufferTime > 0) await this._wait(this.bufferTime);

      try {
        if (this.transitionEffect === 'slide') {
          await this._animate(
            this.overlay,
            [{ transform: 'translateX(0%)' }, { transform: 'translateX(100%)' }],
            { duration: this.duration, easing: this.easing, fill: 'forwards' }
          );
        } else {
          await this._animate(
            this.overlay,
            [{ opacity: 1 }, { opacity: 0 }],
            { duration: this.duration, easing: this.easing, fill: 'forwards' }
          );
        }
      } catch (_) {
        // ignore; we'll force-hide below
      } finally {
        this._hideAtC();
        requestAnimationFrame(() => this._parkAtAAndHide());
      }
    }
  }

  window.TransitionManager = TransitionManager;
})();