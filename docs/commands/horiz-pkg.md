# horiz-pkg コマンド リファレンス

`horiz-pkg` は HorizOS のユーザーランドにおける中核的なパッケージマネージャー兼ダウンローダーである。依存ライブラリを一切持たない（Zero-Dependency）設計でありながら、独自の TLS 1.3 クライアントおよびマニフェストデータベースを内蔵しており、安全にパッケージのダウンロード・検証・インストール・一覧表示・削除を行うことができる。

## サブコマンド一覧

### 1. インストール (`install`)

```bash
horiz-pkg install -u https://example.com/myapp.bin -n myapp -p /bin/pkg.pub
```
- TLS 1.3 通信によるセキュアなダウンロード。
- SHA-512 ハッシュ計算と Ed25519 署名検証。
- 一時ファイル経由の原子的な配置（TOCTOU 対策）。
- インストール完了後、`/var/db/horiz-pkg/manifest.db` にパッケージメタデータを記録。

### 2. インストール済みパッケージ一覧表示 (`list` / `status`)

```bash
horiz-pkg list
```
- `/var/db/horiz-pkg/manifest.db` から登録済みパッケージ名、ステータス、配置パス、URL を一覧表示。

### 3. パッケージの削除 (`remove`)

```bash
horiz-pkg remove myapp
```
- マニフェストデータベースから指定パッケージの情報を読み込み、配置されているバイナリを安全に削除してマニフェストを更新。

## オプション引数

- `-u`, `--url <URL>`: パッケージバイナリの URL。
- `-n`, `--name <NAME>`: 保存先バイナリ名。
- `-p`, `--pubkey <PATH>`: 検証用 Ed25519 公開鍵。
- `--trust <CA_PEM_PATH>`: TLS 検証用ルート証明書。
