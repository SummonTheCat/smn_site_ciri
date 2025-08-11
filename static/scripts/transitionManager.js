
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
     * @param {boolean} [options.initialHold=false]        - if true, do NOT auto-run B→C on load
     * @param {number} [options.initialHoldTimeoutMs=6000] - failsafe to avoid stuck overlay
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
      sessionKey = '__tm_pending_out__',
      initialHold = false,
      initialHoldTimeoutMs = 6000,
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
      this._initialHeld = !!initialHold;
      this._initialHoldTimer = null;
      this._initialHoldTimeoutMs = initialHoldTimeoutMs;

      this._setupOverlayStyles();
      this._installResizeHandler();
      this._installLifecycleHandlers();

      if (this._initialHeld) {
        // Failsafe: never let the overlay stay forever if something goes wrong.
        this._initialHoldTimer = setTimeout(() => {
          if (this._initialHeld) this.releaseInitialHold();
        }, Math.max(0, this._initialHoldTimeoutMs));
      } else {
        // On classic loads, play B→C (then park at A).
        // On bfcache restores, pageshow handler will handle it.
        queueMicrotask(() => this._animateOutThenHide());
      }
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
        if (this.bufferTime > 0) await this._wait(this.bufferTime);
        sessionStorage.setItem(this.sessionKey, '1');
        window.location.assign(url);
      } finally {
        this._isAnimating = false;
      }
      return false;
    }

    /** Public: called by app when initial batch is ready; runs B→C now. */
    async releaseInitialHold() {
      if (!this._initialHeld) return;
      this._initialHeld = false;
      if (this._initialHoldTimer) {
        clearTimeout(this._initialHoldTimer);
        this._initialHoldTimer = null;
      }
      await this._animateOutThenHide();
    }

    /** Helper for inline onclick on anchors. */
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
      window.addEventListener('pageshow', (e) => {
        const pending = sessionStorage.getItem(this.sessionKey) === '1';
        if (e.persisted || pending) {
          this._animateOutThenHide().finally(() => {
            sessionStorage.removeItem(this.sessionKey);
          });
        } else if (!this._initialHeld) {
          this._parkAtAAndHide();
        }
      });

      window.addEventListener('pagehide', () => {
        try { this.overlay.getAnimations().forEach(a => a.cancel()); } catch (_) {}
      });

      window.addEventListener('popstate', () => {
        requestAnimationFrame(() => this._parkAtAAndHide());
      });

      document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible' && !this._initialHeld) {
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
        el.style.opacity = '0';
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