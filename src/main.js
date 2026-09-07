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
  let state = false;

  async function refresh() {
    state = await invoke(getCmd);
    trackEl.classList.toggle('on', state);
    trackEl.setAttribute('aria-checked', String(state));
  }

  async function toggle() {
    state = !state;
    trackEl.classList.toggle('on', state);
    trackEl.setAttribute('aria-checked', String(state));
    await invoke(setCmd, { enabled: state });
  }

  trackEl.addEventListener('click', toggle);
  trackEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); }
  });

  refresh();
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

  // Copy TOTP
  setupCopyTotp();

  // Auto-submit toggle
  setupToggle(el('toggle-auto-submit'), 'get_auto_submit', 'set_auto_submit');

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
