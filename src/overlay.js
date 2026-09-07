const isTauri = typeof window.__TAURI_INTERNALS__ !== 'undefined';

async function invoke(cmd, args) {
  if (isTauri) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke(cmd, args);
  }
  return null;
}

document.addEventListener('DOMContentLoaded', () => {
  const btnAutofill = document.getElementById('btn-autofill');
  const btnClose = document.getElementById('btn-close');
  const totpPreview = document.getElementById('totp-preview');

  async function updateTotp() {
    try {
      const res = /** @type {{code: string, remaining_seconds: number}} */ (await invoke('generate_totp'));
      if (res && res.code) {
        totpPreview.textContent = `TOTP: ${res.code} (${res.remaining_seconds}s)`;
      }
    } catch {
      totpPreview.textContent = 'パスワードのみ入力';
    }
  }

  updateTotp();
  setInterval(updateTotp, 1000);

  btnAutofill.addEventListener('click', async () => {
    try {
      await invoke('trigger_autofill');
      if (isTauri) {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        getCurrentWebviewWindow().hide();
      }
    } catch (e) {
      alert(`自動入力エラー: ${e}`);
    }
  });

  btnClose.addEventListener('click', async () => {
    if (isTauri) {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      getCurrentWebviewWindow().hide();
    }
  });
});
