# FF14 QuickAuth ⚔️

FINAL FANTASY XIV 公式ランチャー専用の軽量・高機能な認証補完マネージャー（Windowsデスクトップアプリ）です。  
ランチャー起動時にオーバーレイUIが表示され、**ワンクリックまたはショートカット操作でパスワードおよびワンタイムパスワード（TOTP）を安全に自動入力**します。

---

## ⚠️ 免責事項 (Disclaimer)

* 本ツールは個人のファンによって作成された**非公式のサードパーティ製便利ツール**です。
* 株式会社スクウェア・エニックス（SQUARE ENIX CO., LTD.）とは一切関係ありません。
* 「FINAL FANTASY」、「ファイナルファンタジー」、「SQUARE ENIX」および関連する名称・ロゴは、日本およびその他の国における株式会社スクウェア・エニックスの商標または登録商標です。
* 本ツールはゲーム本体（`ffxiv_dx11.exe`）のプロセスやメモリへの介入・改変は一切行わず、ランチャー画面（`ffxivlauncher.exe` / `ffxivboot.exe`）への標準的なWin32キー入力送信のみを行います。

---

## ✨ 主な機能

* **🚀 ワンタッチ自動入力**
  * FF14ランチャーの起動をバックグラウンドで自動検知し、ランチャー直上にフローティングオーバーレイを表示。
  * 既存文字列の全選択消去（`Ctrl+A` -> `Backspace`）後に、パスワードおよびTOTPを正しく自動入力します。
* **🔐 セキュアな認証情報管理**
  * 入力したパスワードおよびTOTPシードは、Windows標準の暗号化基盤である **Windows 資格情報マネージャー (DPAPI)** を使用してローカルに安全に保存されます。
  * メモリ上のパスワード・シードデータは使用後に即時ゼロ埋め消去（Zeroize）されます。
* **📱 Google Authenticator / QRコード解析**
  * Google Authenticator等のエクスポートQRコード画像（`otpauth-migration://`）の読み込み、ドラッグ＆ドロップ、クリップボード画像貼り付け解析に対応。
* **⚙️ Enter 自動ログイン & スタートアップ連携**
  * 入力完了後の Enter 自動送信トグル機能（設定は保存されます）。
  * Windowsログイン時の自動起動（スタートアップ登録）を簡単なコマンド（`mise run startup:enable`）で管理可能。

---

## 🛠️ 開発・ビルド手順

### 前提条件
* **OS**: Windows 10 / 11 (x64)
* **Rust**: `stable` (1.77.2以上)
* **Node.js**: `v20` 以上
* **mise** (タスクランナー)

### ビルドと実行

```bash
# リポジトリのクローン
git clone https://github.com/your-username/ff14-quickauth.git
cd ff14-quickauth

# 依存関係のインストール
npm install

# 開発用ローカル起動 (Tauri dev)
npm run dev

# リリースビルド & スタートメニュー登録
mise run install

# Windowsログイン時自動起動の有効化
mise run startup:enable
```

### 📦 自動リリース配信 (GitHub Actions)

バージョンタグを作成して GitHub へ Push すると、GitHub Actions がクリーンな Windows 環境で自動ビルドを行い、GitHub Releases ページへ **インストーラー版 (`.exe`)** および **ポータブル版 (`.zip`)** を自動公開します。

```bash
git tag v0.1.0
git push origin v0.1.0
```

---

## 🏗️ アーキテクチャ

* **GUI / Frontend**: HTML5, CSS3, JavaScript (Vite + WebView2)
* **Backend**: Rust 2021 (Tauri v2)
* **Win32 Integration**: `windows-rs` (SendInput, SetWinEventHook, GetForegroundWindow)
* **Storage**: `keyring` (Windows Credential Manager / DPAPI)

---

## 📄 ライセンス

本プロジェクトは [MIT License](LICENSE) のもとで公開されています。
