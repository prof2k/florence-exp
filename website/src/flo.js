// flo.js — florence bidirectional patcher client
//
// Config (set before loading this script):
//   window.__FLO__ = { file: 'index.html', patcher: 'http://localhost:3001' }

(function () {
  const config = window.__FLO__ || {};
  const PATCHER = config.patcher || 'http://localhost:3001';
  const FILE = config.file || 'index.html';

  function init() {
    document.querySelectorAll('[data-flo-id]').forEach(attachEditor);
  }

  function attachEditor(el) {
    el.setAttribute('contenteditable', 'true');
    el.style.outline = 'none';

    el.addEventListener('focus', () => {
      el.dataset.floOriginal = el.innerHTML;
    });

    el.addEventListener('blur', () => {
      const next = el.innerHTML;
      if (next === el.dataset.floOriginal) return;

      const id = el.dataset.floId;
      const file = el.dataset.floFile || FILE;
      send({ file, id, content: next });
    });

    el.addEventListener('keydown', (e) => {
      const mod = e.ctrlKey || e.metaKey;

      // Enter → line break instead of new block element
      if (e.key === 'Enter') {
        e.preventDefault();
        const sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) return;
        const range = sel.getRangeAt(0);
        range.deleteContents();
        const br = document.createElement('br');
        range.insertNode(br);
        // move cursor after the <br>
        range.setStartAfter(br);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
        return;
      }

      // Escape → revert and exit
      if (e.key === 'Escape') {
        el.innerHTML = el.dataset.floOriginal ?? el.innerHTML;
        el.blur();
        return;
      }

      // Cmd/Ctrl+S → force save
      if (mod && e.key === 's') {
        e.preventDefault();
        el.blur();
        return;
      }

      // Cmd/Ctrl+B → bold (browser handles, we just let it through)
      // Cmd/Ctrl+I → italic
      // Cmd/Ctrl+U → underline
    });
  }

  async function send(data) {
    try {
      const res = await fetch(`${PATCHER}/patch`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      const json = await res.json();
      if (!json.ok) console.error('[flo] patch failed:', json.error);
    } catch (e) {
      console.error('[flo] patcher unreachable:', e);
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
