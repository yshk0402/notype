# TASK.md - notype MVP 実装タスク（マイルストーン詳細版）

> 更新日: 2026-02-13  
> チェック状態の基準:
> - 実装タスク: `[x]` は「コード実装済み（未検証含む）」
> - DoD/検証/AC: 実機検証完了時のみ `[x]`

## 0. 目的
`AGENTS.md` の固定仕様を実装へ落とし込み、`AC-01`〜`AC-08` を満たす MVP を完成させる。  
本タスクは **AC駆動** で進め、各マイルストーンの完了判定（DoD）を満たした時点で次に進む。
`AC-09`〜`AC-11` は実装維持とし、実機検証完了は次フェーズで扱う。

## 1. スコープ

### In Scope（MVP）
- 録音開始/停止
- ローカル STT 非同期実行
- 結果表示
- `Type` / `Copy`
- Auto-type ON/OFF
- 常時最前面ピル + ドラッグ移動
- CLI (`notype`, `notype --settings`, `notype --quit`)
- D-Bus 単一インスタンス制御
- ログイン時自動起動
- エラー時 `Idle` 復帰、一時ファイル清掃

### Out of Scope（MVP外）
- LLM 後処理の本実装
- `ydotool` 本対応
- アプリ内グローバルホットキー
- deb/Flatpak 配布

## 2. 実装前提（確定）
- スタック: `Rust + Tauri`
- タスク軸: `AC駆動`
- モデル配布: 初回起動時 DL（default `small`）
- IPC: `D-Bus`
- 注入主経路: `wtype`
- UI: 常駐ピル + 別設定ウィンドウ
- 閉じる操作デフォルト: 非表示（常駐継続）

## 3. 契約（公開インターフェース）

### 3.1 CLI 契約
- `notype`
  - 未起動: 新規起動
  - 起動済み: メインピル前面化
- `notype --settings`
  - 未起動: 起動 + 設定画面表示
  - 起動済み: 設定画面前面化
- `notype --quit`
  - 起動済みインスタンスへ終了要求送信

### 3.2 IPC（D-Bus）契約
- `ShowMain`
- `ShowSettings`
- `Quit`

### 3.3 状態機械契約
- 許可状態: `Idle | Recording | Processing | Ready`
- 失敗時: 全経路で `Idle` に復帰

### 3.4 設定スキーマ契約
- `max_record_seconds: number`
- `model: "small" | "medium"`
- `auto_type: boolean`
- `text_cleanup: boolean`
- `llm_postprocess_enabled: boolean`（拡張予約）
- `llm_provider: string`（拡張予約）
- `realtime_enabled: boolean`
- `partial_autotype_mode: "replace"`（experimental）
- `pill_position?: { x: number; y: number }`

### 3.5 Tauri コマンド契約
- `get_pill_position`
- `set_pill_position({ x, y })`

### 3.6 イベント契約
- `notype://transcript`
- `notype://error`
- `notype://model-download`（`progress`, `status`, `message`）

## 4. マイルストーン

---

## M1. Foundation（アプリ骨格・契約固定）

### 実装タスク
- [x] Rust + Tauri プロジェクト初期化
- [x] モジュール境界定義
  - `core/audio`
  - `core/stt`
  - `core/inject`
  - `core/ipc`
  - `core/config`
  - `core/state`
- [x] アプリ内イベント定義
  - state 変更通知
  - STT 結果通知
  - エラー通知
- [x] 共通エラー型定義
  - UI表示メッセージ（短文）
  - ログ詳細（原因・文脈）
- [x] 設定 I/O 雛形
  - デフォルト値投入
  - 不正値時フォールバック
  - 破損ファイル時自動再生成

### 成果物
- [x] 初期起動可能な Tauri アプリ（依存取得後のビルド確認待ち）
- [x] 状態と設定を扱う最小 API
- [x] エラーハンドリング方針ドキュメント（短文）

### DoD
- [ ] 起動時に `Idle` で初期化される
- [ ] 設定ファイル欠損/破損でも起動継続

### 検証
- [ ] 正常設定ロード
- [ ] 欠損設定ロード
- [ ] 破損設定ロード

---

## M2. Pill UI（常駐ピル最小体験）

### 実装タスク
- [x] 2アイコン固定 UI（設定 / 音声）実装
- [x] 常時最前面設定
- [x] ドラッグ移動実装
- [x] 位置保存と復元
- [x] 状態表示（Idle/Recording/Processing/Ready）

### 成果物
- [x] 最小ピル UI（固定構成）
- [x] 位置永続化（設定保存）

### DoD
- [ ] ピルが常時最前面を維持
- [ ] ドラッグ操作が滑らか
- [ ] 再起動後に前回位置へ復元

### 検証
- [ ] AC-04（最前面 + ドラッグ）
- [ ] 画面解像度変更時の位置補正

---

## M3. Audio Capture（録音制御）

### 実装タスク
- [x] default マイクで録音開始/停止
- [x] `max_record_seconds` で自動停止
- [x] 録音中の重複起動抑止（排他）
- [x] 音声一時ファイル生成
- [x] 成功/失敗/中断で tmp 清掃
- [x] 状態遷移統合（Idle -> Recording -> Processing）

### 成果物
- [x] 録音サービス層
- [x] tmp 管理ユーティリティ

### DoD
- [ ] 連続録音でリークしない
- [ ] 録音停止後に必ず Processing に遷移

### 検証
- [ ] 長押し/連打
- [ ] 録音中停止
- [ ] デバイス未接続エラー

---

## M4. STT Pipeline（非同期文字起こし）

### 実装タスク
- [x] Whisper 実行ラッパー（`small`/`medium`）
- [x] 初回起動時 `small` モデル DL
- [x] DL 進捗表示イベント
- [x] STT 非同期実行（UI スレッド非ブロック）
- [x] 録音中 partial（300-700ms目標）イベント配信
- [x] `text_cleanup` 実装（空白/改行整形）
- [x] STT 失敗時の `Idle` 復帰

### 成果物
- [x] STT サービス層
- [x] モデル管理サービス（存在確認/DL/再試行）

### DoD
- [ ] 録音完了後にテキスト結果を返せる
- [ ] 処理中でも UI が応答
- [ ] 録音中 partial が定期更新される
- [ ] 失敗後に即再試行可能

### 検証
- [ ] AC-03（UI応答）
- [ ] AC-06（LLM OFF + ネットワーク遮断）
- [ ] AC-09（partial 300-700ms）
- [ ] モデル欠損/破損時リカバリ

---

## M5. Injection UX（Type / Copy / Auto-type）

### 実装タスク
- [x] 結果テキスト表示領域
- [x] `Type` ボタン実装
- [x] `Copy` ボタン実装
- [x] `auto_type` ON/OFF 分岐
- [x] `wtype` 実行経路実装
- [x] partial 都度置換注入（experimental）
- [x] 置換失敗時の同セッション final-only 降格
- [x] 注入失敗時の UX
  - 結果テキスト保持
  - `Copy` 代替導線
  - 明確なエラーメッセージ

### 成果物
- [x] 入力注入サービス（wtype）
- [x] 結果操作 UI（Type/Copy）

### DoD
- [ ] AC-01, AC-02 を満たす
- [ ] AC-10 を満たす
- [ ] 注入失敗でも結果消失がない

### 検証
- [ ] 日本語/英数混在テキスト注入
- [ ] 注入先未フォーカス時の失敗挙動

---

## M6. IPC & CLI（単一インスタンス制御）

### 実装タスク
- [x] D-Bus サービス実装
- [x] 単一インスタンス保証
- [x] `ShowMain` 実装
- [x] `ShowSettings` 実装
- [x] `Quit` 実装
- [x] CLI エントリ実装
  - `notype`
  - `notype --settings`
  - `notype --quit`
- [x] 設定ウィンドウ前面化制御
- [x] IPC 前面化/終了処理の失敗ログ追加

### 成果物
- [x] IPC サービス層
- [x] CLI 実行バイナリ挙動

### DoD
- [ ] AC-07, AC-08 を満たす
- [ ] 多重起動しない

### 検証
- [ ] 未起動時動作
- [ ] 起動済み時動作
- [ ] 異常終了後の再起動

---

## M7. Startup & Hardening（運用品質）

### 実装タスク
- [x] `.desktop` エントリ作成
- [x] ログイン時自動起動登録導線
- [x] 閉じる操作デフォルトを「非表示（常駐）」に固定
- [x] 失敗系シナリオ4件の手順化
- [x] AC-01〜08 総合 E2E 手順化

### 成果物
- [x] 配布時に利用できる `.desktop` 定義
- [x] 手動テスト手順書（MVP）

### DoD
- [ ] AC-05 を満たす
- [ ] 全 AC の再現可能手順が揃う

### 検証
- [ ] 再ログイン後自動起動
- [ ] 録音中クラッシュ復旧
- [ ] ネットワーク遮断時安定性

## 5. 横断タスク（全マイルストーン共通）
- [ ] すべてのエラー経路で `Idle` 復帰保証
- [ ] UI スレッド非ブロック徹底
- [ ] ログ粒度統一（録音/STT/注入/IPC）
- [ ] 音声外部送信禁止の境界維持
- [ ] tmp 清掃漏れゼロ

## 6. 受け入れ基準（チェックリスト）
- [ ] AC-01: VS Code に 10 秒発話し、Auto-type ON で入力される
- [ ] AC-02: Auto-type OFF で結果表示後 `Type` 押下時のみ入力される
- [ ] AC-03: Recording / Processing 中も UI 応答性を維持
- [ ] AC-04: 最前面表示とドラッグ移動ができる
- [ ] AC-05: ログイン後に自動起動する
- [ ] AC-06: LLM OFF + ネットワーク遮断で動作する
- [ ] AC-07: `notype --settings` で設定表示または前面化
- [ ] AC-08: `notype --quit` で安全終了できる
- [ ] AC-09: 録音中に partial が 300-700ms で更新される（次フェーズで実機検証）
- [ ] AC-10: partial 置換注入が動作し、失敗時は同セッションで final-only に降格（次フェーズで実機検証）
- [ ] AC-11: 音声データが外部送信されない（LLM OFF）（次フェーズで実機検証）

## 7. 失敗系テスト（必須）
- [ ] STT 失敗時に明確なエラー表示 + `Idle` 復帰
- [ ] 注入失敗時に結果保持 + `Copy` 継続利用
- [ ] 録音中クラッシュ後に再起動で正常復帰
- [ ] ネットワーク遮断時の挙動が安定

## 8. 推奨実行順
1. M1 Foundation
2. M2 Pill UI
3. M3 Audio Capture
4. M4 STT Pipeline
5. M5 Injection UX
6. M6 IPC & CLI
7. M7 Startup & Hardening

## 9. 実装上の禁止事項
- [ ] UI スレッドで録音/STT/注入を直接実行しない
- [ ] エラー時に `Idle` へ戻さず終了しない
- [ ] tmp ファイルを残置しない
- [ ] 音声データを外部 API に送信しない

## 10. AC-01〜AC-08 手動 E2E 手順
1. 依存確認: `which arecord wtype wl-copy whisper-cli` がすべて成功。
2. 起動: `pnpm tauri dev` で起動し、ピルが表示されることを確認。
3. AC-01: `auto_type=true` で VS Code にフォーカスして 10 秒録音、停止後に注入されること。
4. AC-02: `auto_type=false` で録音停止後、`Type` 押下時のみ注入されること。
5. AC-03: Recording/Processing 中にピル操作と設定UI操作が応答すること。
6. AC-04: ピルが最前面かつドラッグで移動し、再起動後も位置が復元されること。
7. AC-05: `./scripts/install-autostart.sh` 実行後、再ログインで自動起動すること。
8. AC-06: ネットワーク遮断状態で LLM OFF のまま録音→STT→表示が成立すること。
9. AC-07: `notype --settings` で未起動時は起動+設定表示、起動済み時は前面化すること。
10. AC-08: `notype --quit` で安全終了し、その後 `notype` で再起動できること。

## 11. 失敗系テスト手順
1. STT 失敗: モデル/whisper-cli を一時的に無効化し、エラー表示後 `Idle` に戻ること。
2. 注入失敗: `wtype` を失敗させ、結果テキストが保持され `Copy` が使えること。
3. 録音中クラッシュ: 録音中にプロセス停止後、再起動で再録音できること。
4. ネットワーク遮断: 初回DL後に遮断し、既存モデルで動作継続すること。

## 12. 現在の検証ブロッカー
- `cargo test --manifest-path src-tauri/Cargo.toml` は 2026-02-13 に実行完了（5 tests passed）。
- AC/DoD のチェック更新は、Wayland + GNOME 実機での手動検証後に実施する。
