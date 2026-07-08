// Thin JS interop shim: the only piece of this app that stays hand-written
// JavaScript, because Tesseract.js (handwriting OCR) has no Rust/WASM port.
// Rust calls `window.spellOcr(canvas)` (see src/drawing.rs) and awaits the
// returned promise for the recognized text.

let workerPromise = null;

function loadScript(src) {
  return new Promise((resolve, reject) => {
    const s = document.createElement('script');
    s.src = src;
    s.onload = resolve;
    s.onerror = () => reject(new Error('load failed: ' + src));
    document.head.appendChild(s);
  });
}

async function ensureWorker() {
  if (workerPromise) return workerPromise;
  workerPromise = (async () => {
    if (typeof Tesseract === 'undefined') {
      await loadScript('https://cdn.jsdelivr.net/npm/tesseract.js@5/dist/tesseract.min.js');
    }
    const worker = await Tesseract.createWorker('eng', 1, {
      workerPath: 'https://cdn.jsdelivr.net/npm/tesseract.js@5/dist/worker.min.js',
      corePath: 'https://cdn.jsdelivr.net/npm/tesseract.js-core@5',
      langPath: 'https://tessdata.projectnaptha.com/4.0.0',
    });
    await worker.setParameters({
      tessedit_pageseg_mode: '8',
      tessedit_char_whitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz',
    });
    return worker;
  })();
  return workerPromise;
}

window.spellOcr = async function spellOcr(canvas) {
  const worker = await ensureWorker();
  const { data } = await worker.recognize(canvas);
  // confidence is Tesseract's own 0-100 mean-confidence score for the
  // recognized text — the caller uses it to decide whether to trust this
  // read automatically or ask the player to confirm/fix it first.
  return { text: (data && data.text) || '', confidence: (data && data.confidence) || 0 };
};
