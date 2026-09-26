# FF14 QuickAuth — 開発・ビルドガイド 🛠️

本ドキュメントは、FF14 QuickAuth の開発環境構築、ローカルビルド、およびリリース自動化手順を解説します。

---

## 💻 開発環境の前提条件

* **OS**: Windows 10 / 11 (x64)
* **Rust**: `stable` (1.77.2 以上)
* **Node.js**: `v20` 以上 (LTS推奨)
* **mise**: CLIタスクランナー (推奨)

---

## 🚀 ローカルでの開発・ビルド手順

### 1. リポジトリのクローンと依存関係のインストール

```bash
git clone https://github.com/your-username/ff14-quickauth.git
cd ff14-quickauth

npm install
```

### 2. 開発用ローカル起動 (Tauri Dev Mode)

```bash
npm run dev
```

### 3. アプリのビルド・インストール・再インストール (mise)

```bash
# アプリをリリースビルドし、Windowsスタートメニューに登録/上書き更新（再インストール）します
mise run install
```

### 4. アプリのアンインストール (mise)

```bash
# Windowsスタートメニューからショートカットを削除します
mise run uninstall

# スタートアップフォルダから自動起動ショートカットを削除します
mise run startup:disable
```

* **認証情報の完全削除**:  
  アプリ画面内の **「🗑（削除）」** ボタンを押すか、Windowsの「資格情報マネージャー ➜ Windows 資格情報」から `ff14-quickauth` エントリを削除します。

### 5. Windows ログイン時自動起動の切り替え (mise)

```bash
# スタートアップフォルダに自動起動ショートカットを登録（有効化）
mise run startup:enable

# スタートアップフォルダから自動起動ショートカットを削除（無効化）
mise run startup:disable
```

### 6. 単体テストの実行

```bash
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 🛠️ `mise` CLI タスク一覧

| 操作 | コマンド | 説明 |
|---|---|---|
| **ビルド** | `mise run build` | Tauriリリースバイナリおよびインストーラーパッケージを生成します。 |
| **インストール / 再インストール** | `mise run install` | リリースビルドを行い、スタートメニューのショートカットを最新に更新・再インストールします。 |
| **アンインストール** | `mise run uninstall` | スタートメニューからショートカットを削除します。 |
| **自動起動の有効化** | `mise run startup:enable` | Windowsスタートアップフォルダにショートカットを登録し、ログイン時自動起動を有効化します。 |
| **自動起動の無効化** | `mise run startup:disable` | Windowsスタートアップフォルダから自動起動ショートカットを削除します。 |

---

## 📦 自動リリース配信 (GitHub Actions)

新しいバージョンをリリースする際は、セムバー（Semantic Versioning）に従って git タグを作成し、GitHub へ push します。  
GitHub Actions がクリーンな Windows 環境（`windows-latest`）で自動ビルドを行い、GitHub Releases ページへ成果物を自動公開します。

```bash
# タグの作成
git tag v0.1.0

# タグの送信 (CIが起動します)
git push origin v0.1.0
```

---

## 🏗️ 技術スタック & アーキテクチャ

* **GUI / Frontend**: HTML5, CSS3, JavaScript (Vite + WebView2)
* **Backend**: Rust 2021 (Tauri v2)
* **Win32 Integration**: `windows-rs` (SendInput, SetWinEventHook, GetForegroundWindow)
* **Storage**: `keyring` (Windows Credential Manager / DPAPI)
* **Encryption / Security**: DPAPI (Windows Native), `secrecy`, `zeroize`
