// scripts/linkReplacer.js

(() => {
  // Keep track of all managed anchors for cross-reset behavior
  const managedAnchors = new Set();

  /**
   * Build a mailto URL with optional subject/body.
   */
  function buildMailto(email, subject, body) {
    const params = new URLSearchParams();
    if (subject) params.set('subject', subject);
    if (body) params.set('body', body);
    const query = params.toString();
    return `mailto:${email}${query ? `?${query}` : ''}`;
  }

  /**
   * Save the initial state so we can restore it later.
   */
  function captureInitialState(anchor) {
    if (anchor.dataset._initCaptured === '1') return;

    anchor.dataset.defaultText = (anchor.textContent || '').trim();
    anchor.dataset.defaultHref = anchor.getAttribute('href') || '#';
    anchor.dataset.defaultTitle = anchor.getAttribute('title') || '';
    anchor.dataset.defaultTarget = anchor.getAttribute('target') || '';
    anchor.dataset.defaultRel = anchor.getAttribute('rel') || '';

    anchor.dataset._initCaptured = '1';
  }

  /**
   * Restore a contact anchor back to its default/unrevealed state.
   */
  function resetContactLink(anchor) {
    if (!anchor || anchor.dataset._initCaptured !== '1') return;

    anchor.textContent = anchor.dataset.defaultText || 'Contact';
    const href = anchor.dataset.defaultHref || '#';
    anchor.setAttribute('href', href);

    // Restore or remove attributes to their defaults
    if (anchor.dataset.defaultTitle) {
      anchor.setAttribute('title', anchor.dataset.defaultTitle);
    } else {
      anchor.removeAttribute('title');
    }

    if (anchor.dataset.defaultTarget) {
      anchor.setAttribute('target', anchor.dataset.defaultTarget);
    } else {
      anchor.removeAttribute('target');
    }

    if (anchor.dataset.defaultRel) {
      anchor.setAttribute('rel', anchor.dataset.defaultRel);
    } else {
      anchor.removeAttribute('rel');
    }

    // Clear state flags
    anchor.removeAttribute('data-revealed');
    anchor.removeAttribute('data-copied');
  }

  /**
   * Reveal the "real" value on the link (text + href + title), mark revealed.
   * Returns the final href that was applied (if any).
   */
  function revealContact(anchor, mode) {
    if (anchor.dataset.revealed === '1') {
      return anchor.getAttribute('href') || null;
    }

    let finalHref = null;

    if (mode === 'email') {
      const email = anchor.dataset.email || '';
      const subject = anchor.dataset.subject || '';
      const body = anchor.dataset.body || '';

      finalHref = buildMailto(email, subject, body);

      anchor.textContent = email || 'Email';
      anchor.setAttribute('href', finalHref);
      anchor.removeAttribute('target');
      anchor.title = email;
    } else if (mode === 'discord') {
      const handle = anchor.dataset.discord || 'Discord';
      const url = anchor.dataset.discordUrl || '';

      anchor.textContent = handle;

      if (url) {
        finalHref = url;
        anchor.setAttribute('href', url);
        anchor.setAttribute('target', '_blank');
        anchor.setAttribute('rel', 'noopener');
      } else {
        // No URL provided—leave href as "#" but add title and copy on first click
        anchor.setAttribute('href', '#');
        anchor.title = handle;
      }
    }

    anchor.dataset.revealed = '1';
    return finalHref;
  }

  /**
   * Immediately activates the contact method after reveal:
   * - email: opens the user's mail app with mailto
   * - discord: opens the provided URL in a new tab (if any), otherwise copies the handle
   */
  async function activateContact(anchor, mode, finalHref) {
    if (mode === 'email') {
      // Use direct navigation; do NOT run the page transition for mailto
      window.location.href = finalHref;
    } else if (mode === 'discord') {
      const url = finalHref || '';
      if (url) {
        // Open in a new tab; transitions are not applied for external tabs
        window.open(url, '_blank', 'noopener');
      } else {
        // Nothing to open—copy the handle and inform the user
        const handle = anchor.dataset.discord || '';
        try {
          if (navigator.clipboard && handle) {
            await navigator.clipboard.writeText(handle);
            anchor.setAttribute('data-copied', '1');
            const text = handle;
            anchor.textContent = `${text} (copied)`;
            setTimeout(() => {
              if (anchor.getAttribute('data-copied') === '1') {
                anchor.textContent = text;
                anchor.removeAttribute('data-copied');
              }
            }, 1500);
          }
        } catch {
          // Silent fallback
        }
      }
    }
  }

  /**
   * Reset all other managed contact links (except the current one).
   */
  function resetOthers(current) {
    managedAnchors.forEach(a => {
      if (a !== current) resetContactLink(a);
    });
  }

  /**
   * Initialize a single contact anchor with reveal-then-activate behavior.
   * First click: reset others, reveal this, then immediately activate.
   * Subsequent clicks: behave like a normal link (mailto, or external Discord URL).
   */
  function initContactLink(anchor) {
    if (!anchor) return;
    captureInitialState(anchor);
    managedAnchors.add(anchor);

    const mode = (anchor.dataset.mode || '').toLowerCase(); // 'email' | 'discord'

    anchor.addEventListener('click', (e) => {
      // Already revealed? Let browser handle naturally.
      if (anchor.dataset.revealed === '1') return;

      e.preventDefault();

      // Reset all others before revealing this one
      resetOthers(anchor);

      const finalHref = revealContact(anchor, mode);

      // Slight delay so the label swap is visible
      setTimeout(() => { activateContact(anchor, mode, finalHref); }, 30);
    }, { passive: false });
  }

  /**
   * Initialize multiple contact links by selector.
   */
  function initContactLinks(selector) {
    document.querySelectorAll(selector).forEach(initContactLink);
  }

  // Expose initializer
  window.initContactLinks = initContactLinks;
})();
