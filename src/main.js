import jsQR from 'jsqr';

/**
 * FF14 Companion — Main Frontend
 *
 * Responsibilities:
 *   1. Load/save credentials via Tauri commands
 *   2. Poll TOTP code every second and render countdown ring
 *   3. Tab navigation between Credential and Settings panels
 *   4. Toggle switches for settings
 *   5. Enable "manual fill" button when credentials are saved
 */

// ── Tauri invoke bridge (falls back to mock in browser dev mode) ─────────────
const isTauri = typeof window.__TAURI_INTERNALS__ !== 'undefined';

/**
 * @param {string} cmd
 * @param {Record<string, unknown>} [args]
 * @returns {Promise<unknown>}
 */
async function invoke(cmd, args) {
  if (isTauri) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke(cmd, args);
  }
  // Dev-mode stub — returns mock data so the UI is browsable without Tauri
  const stubs = {
    get_credential: () => null,
    save_credential: () => undefined,
    delete_credential: () => undefined,
    generate_totp: () => ({ code: '123456', remaining_seconds: 20 }),
    get_auto_submit: () => false,
    set_auto_submit: () => undefined,
    get_autostart: () => false,
    set_autostart: () => undefined,
    check_for_update: () => ({ should_update: false, version: null, body: null }),
    install_update: () => undefined,
    parse_totp_input: () => [
      { secret_base32: 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ', name: 'FF14 Main', issuer: 'Square Enix' }
    ],
  };
  const fn = stubs[cmd];
  if (!fn) throw new Error(`Unknown command: ${cmd}`);
  return fn(args);
}

// ── State ─────────────────────────────────────────────────────────────────────
let totpInterval = null;
let hasCredential = false;

// ── DOM helpers ───────────────────────────────────────────────────────────────
/** @param {string} id @returns {HTMLElement} */
const el = (id) => document.getElementById(id);

/**
 * Show a status message.
 * @param {HTMLElement} el
 * @param {'success'|'error'} type
 * @param {string} message
 */
function showStatus(statusEl, type, message) {
  statusEl.textContent = message;
  statusEl.className = `status status--${type} visible`;
  setTimeout(() => {
    statusEl.className = 'status';
  }, 4000);
}

// ── TOTP Ring ─────────────────────────────────────────────────────────────────
const CIRCUMFERENCE = 2 * Math.PI * 44; // r=44 → 276.46

function updateTotpRing(code, remainingSeconds) {
  el('totp-code-display').textContent = code;
  el('totp-remaining-display').textContent = `${remainingSeconds} 秒`;

  const progress = el('totp-ring-progress');
  const fraction = remainingSeconds / 30;
  progress.style.strokeDashoffset = CIRCUMFERENCE * (1 - fraction);

  // Turn red when < 5 seconds
  progress.style.stroke = remainingSeconds < 5
    ? 'var(--c-danger)'
    : 'var(--c-gold-400)';
}

async function refreshTotp() {
  try {
    const result = await invoke('generate_totp');
    el('totp-card').style.display = 'block';
    updateTotpRing(result.code, result.remaining_seconds);
  } catch {
    // No TOTP seed — hide the card
    el('totp-card').style.display = 'none';
  }
}

function startTotpPolling() {
  stopTotpPolling();
  refreshTotp();
  totpInterval = setInterval(refreshTotp, 1000);
}

function stopTotpPolling() {
  if (totpInterval) {
    clearInterval(totpInterval);
    totpInterval = null;
  }
}

// ── Load credential into form ─────────────────────────────────────────────────
async function loadCredential() {
  try {
    const cred = await invoke('get_credential');
    if (cred) {
      el('input-account-name').value = cred.account_name;
      el('input-password').value = cred.password;
      el('input-totp-seed').value = cred.totp_seed ?? '';
      hasCredential = true;
      el('btn-manual-fill').disabled = false;
      if (cred.totp_seed) startTotpPolling();
    }
  } catch (e) {
    console.error('Failed to load credential:', e);
  }
}

// ── Save credential ───────────────────────────────────────────────────────────
async function saveCredential(event) {
  event.preventDefault();
  const statusEl = el('status-credential');

  const dto = {
    account_name: el('input-account-name').value.trim(),
    password: el('input-password').value,
    totp_seed: el('input-totp-seed').value.trim() || null,
  };

  try {
    await invoke('save_credential', { dto });
    hasCredential = true;
    el('btn-manual-fill').disabled = false;
    showStatus(statusEl, 'success', '✅ 認証情報を保存しました');
    if (dto.totp_seed) {
      startTotpPolling();
    } else {
      stopTotpPolling();
      el('totp-card').style.display = 'none';
    }
  } catch (e) {
    showStatus(statusEl, 'error', `❌ 保存失敗: ${e}`);
  }
}

// ── Delete credential ─────────────────────────────────────────────────────────
async function deleteCredential() {
  if (!confirm('認証情報を削除しますか？')) return;

  try {
    await invoke('delete_credential');
    el('input-account-name').value = '';
    el('input-password').value = '';
    el('input-totp-seed').value = '';
    hasCredential = false;
    el('btn-manual-fill').disabled = true;
    stopTotpPolling();
    el('totp-card').style.display = 'none';
    showStatus(el('status-credential'), 'success', '🗑 認証情報を削除しました');
  } catch (e) {
    showStatus(el('status-credential'), 'error', `❌ 削除失敗: ${e}`);
  }
}

// ── Manual autofill ───────────────────────────────────────────────────────────
async function triggerManualFill() {
  try {
    // trigger_autofill command: instructs the Rust side to inject credentials
    await invoke('trigger_autofill');
  } catch (e) {
    showStatus(el('status-credential'), 'error', `❌ 入力失敗: ${e}`);
  }
}

// ── Tab navigation ────────────────────────────────────────────────────────────
function switchTab(activeId) {
  const tabs = ['credential', 'settings'];
  tabs.forEach(name => {
    const tab = el(`tab-${name}`);
    const panel = el(`panel-${name}`);
    const isActive = name === activeId;
    tab.classList.toggle('active', isActive);
    tab.setAttribute('aria-selected', String(isActive));
    panel.style.display = isActive ? 'block' : 'none';
  });
}

// ── Toggle switch ─────────────────────────────────────────────────────────────
function setupToggle(trackEl, getCmd, setCmd) {
  const storageKey = setCmd.replace(/^set_/, '');
  let state = false;

  async function syncState() {
    trackEl.classList.toggle('on', state);
    trackEl.setAttribute('aria-checked', String(state));
    try {
      await invoke(setCmd, { enabled: state });
    } catch (e) {
      console.error(`Failed to sync toggle state for ${setCmd}:`, e);
    }
  }

  async function init() {
    try {
      const fetched = await invoke(getCmd);
      if (typeof fetched === 'boolean') {
        state = fetched;
      } else {
        const saved = localStorage.getItem(storageKey);
        state = saved !== null ? saved === 'true' : false;
      }
    } catch {
      const saved = localStorage.getItem(storageKey);
      state = saved !== null ? saved === 'true' : false;
    }
    trackEl.classList.toggle('on', state);
    trackEl.setAttribute('aria-checked', String(state));
  }

  async function toggle() {
    state = !state;
    localStorage.setItem(storageKey, String(state));
    await syncState();
  }

  trackEl.addEventListener('click', toggle);
  trackEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); }
  });

  init();
}

// ── Password visibility toggles ───────────────────────────────────────────────
function setupVisibilityToggle(btnId, inputId) {
  el(btnId).addEventListener('click', () => {
    const input = el(inputId);
    const isHidden = input.type === 'password';
    input.type = isHidden ? 'text' : 'password';
    el(btnId).textContent = isHidden ? '🙈' : '👁';
  });
}

// ── Copy TOTP ─────────────────────────────────────────────────────────────────
function setupCopyTotp() {
  el('btn-copy-totp').addEventListener('click', async () => {
    const code = el('totp-code-display').textContent;
    if (code === '------') return;
    try {
      await navigator.clipboard.writeText(code);
      el('btn-copy-totp').textContent = '✅ コピーしました';
      setTimeout(() => { el('btn-copy-totp').textContent = '📋 コピー'; }, 2000);
    } catch {
      el('btn-copy-totp').textContent = '❌ 失敗';
    }
  });
}

// ── TOTP Seed & QR Parser ──────────────────────────────────────────────────────
async function applyTotpText(text) {
  const statusEl = el('status-credential');
  try {
    const accounts = /** @type {Array<{secret_base32: string, name?: string, issuer?: string}>} */ (
      await invoke('parse_totp_input', { input: text })
    );
    if (!accounts || accounts.length === 0) {
      showStatus(statusEl, 'error', '❌ 有効なTOTPシードを検出できませんでした');
      return;
    }
    const target = accounts[0];
    el('input-totp-seed').value = target.secret_base32;
    if (target.name && !el('input-account-name').value.trim()) {
      el('input-account-name').value = target.name;
    }
    showStatus(statusEl, 'success', `✅ TOTPシードを読み込みました (${target.name || '抽出成功'})`);
  } catch (e) {
    showStatus(statusEl, 'error', `❌ QR/シード解析エラー: ${e}`);
  }
}

function processQrImageFile(file) {
  const reader = new FileReader();
  reader.onload = (e) => {
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement('canvas');
      canvas.width = img.width;
      canvas.height = img.height;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.drawImage(img, 0, 0);
      const imageData = ctx.getImageData(0, 0, img.width, img.height);
      const code = jsQR(imageData.data, imageData.width, imageData.height);
      if (code && code.data) {
        applyTotpText(code.data);
      } else {
        showStatus(el('status-credential'), 'error', '❌ 画像からQRコードを読み取れませんでした');
      }
    };
    img.src = String(e.target.result);
  };
  reader.readAsDataURL(file);
}

function setupQrScanner() {
  const btnScan = el('btn-scan-qr');
  const fileInput = /** @type {HTMLInputElement} */ (el('input-qr-file'));

  btnScan.addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', () => {
    if (fileInput.files && fileInput.files[0]) {
      processQrImageFile(fileInput.files[0]);
    }
    fileInput.value = '';
  });

  // Paste handler for QR image or migration URL
  document.addEventListener('paste', (e) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    for (const item of items) {
      if (item.type.startsWith('image/')) {
        const blob = item.getAsFile();
        if (blob) {
          e.preventDefault();
          processQrImageFile(blob);
          return;
        }
      }
    }

    const pastedText = e.clipboardData?.getData('text');
    if (pastedText && (pastedText.startsWith('otpauth-migration://') || pastedText.startsWith('otpauth://'))) {
      e.preventDefault();
      applyTotpText(pastedText);
    }
  });

  // Drag and drop QR image onto window
  window.addEventListener('dragover', (e) => e.preventDefault());
  window.addEventListener('drop', (e) => {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (files && files[0] && files[0].type.startsWith('image/')) {
      processQrImageFile(files[0]);
    }
  });

  // Auto-parse on seed input blur if starts with otpauth
  el('input-totp-seed').addEventListener('change', () => {
    const val = /** @type {HTMLInputElement} */ (el('input-totp-seed')).value.trim();
    if (val.startsWith('otpauth-migration://') || val.startsWith('otpauth://')) {
      applyTotpText(val);
    }
  });
}

// ── Updater UI ────────────────────────────────────────────────────────────────
function setupUpdaterUI() {
  const btnCheck = el('btn-check-update');
  const btnDoUpdate = el('btn-do-update');
  const statusText = el('update-status-text');
  const actionBox = el('update-action-box');
  const bannerTitle = el('update-banner-title');
  const bannerBody = el('update-banner-body');

  async function check(manual = false) {
    if (manual) statusText.textContent = '更新を確認中...';
    try {
      const res = /** @type {{should_update: boolean, version?: string, body?: string}} */ (
        await invoke('check_for_update')
      );
      if (res && res.should_update) {
        statusText.textContent = `新しいバージョン (v${res.version}) が利用可能です`;
        bannerTitle.textContent = `🎉 バージョン v${res.version} が利用可能です！`;
        bannerBody.textContent = res.body || '最新のアップデートがリリースされています。';
        actionBox.style.display = 'block';
      } else {
        if (manual) statusText.textContent = 'お使いのバージョンは最新です';
        actionBox.style.display = 'none';
      }
    } catch (e) {
      if (manual) statusText.textContent = `更新確認エラー: ${e}`;
      console.warn('Update check failed:', e);
    }
  }

  btnCheck.addEventListener('click', () => check(true));

  btnDoUpdate.addEventListener('click', async () => {
    btnDoUpdate.disabled = true;
    btnDoUpdate.textContent = '⏳ ダウンロード＆アップデート中...';
    try {
      await invoke('install_update');
    } catch (e) {
      btnDoUpdate.disabled = false;
      btnDoUpdate.textContent = '⚡️ 今すぐアップデートして再起動';
      alert(`アップデートに失敗しました: ${e}`);
    }
  });

  // Automatically check on launch
  setTimeout(() => check(false), 2000);
}

// ── Init ──────────────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', async () => {
  // Tab navigation
  el('tab-credential').addEventListener('click', () => switchTab('credential'));
  el('tab-settings').addEventListener('click', () => switchTab('settings'));

  // Form events
  el('form-credential').addEventListener('submit', saveCredential);
  el('btn-delete').addEventListener('click', deleteCredential);
  el('btn-manual-fill').addEventListener('click', triggerManualFill);

  // Password visibility
  setupVisibilityToggle('btn-toggle-pass', 'input-password');
  setupVisibilityToggle('btn-toggle-seed', 'input-totp-seed');

  // Copy TOTP & QR scanner
  setupCopyTotp();
  setupQrScanner();

  // Auto-submit & Autostart toggles
  setupToggle(el('toggle-auto-submit'), 'get_auto_submit', 'set_auto_submit');
  setupToggle(el('toggle-autostart'), 'get_autostart', 'set_autostart');

  // Updater UI
  setupUpdaterUI();

  // Load saved credential
  await loadCredential();

  // Listen for Tauri events (launcher detection)
  if (isTauri) {
    const { listen } = await import('@tauri-apps/api/event');

    // Launcher detected event from Rust backend
    await listen('launcher-detected', () => {
      el('launcher-status').innerHTML = '⬤ <span style="color:var(--c-success)">ランチャー検出</span>';
    });

    await listen('launcher-closed', () => {
      el('launcher-status').textContent = '⬤ ランチャー未検出';
    });
  }
});
