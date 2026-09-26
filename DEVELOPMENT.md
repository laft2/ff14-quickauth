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

### 3. リリースバイナリのビルドとスタートメニュー登録

```bash
# アプリをリリースビルドし、スタートメニューに登録します
mise run install
```

### 4. Windows ログイン時自動起動のテスト

```bash
# スタートアップフォルダに自動起動ショートカットを登録
mise run startup:enable

# 自動起動ショートカットの解除
mise run startup:disable
```

### 5. 単体テストの実行

```bash
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

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
