# ToDo List

現在の実装状況を踏まえ、今後の開発タスクをフェーズ分けしました。

## Phase 1: Publish機能の実装 (双方向通信の確立)
- [x] **Backend:** `publish_message` コマンドの実装 (Rust)
  - [x] コマンドハンドラの定義 (`commands.rs` 作成を検討)
  - [x] MQTTクライアントへの送信ロジック実装 (Mutexまたはチャネルを使用)
- [x] **Frontend:** Publish機能の呼び出しテスト
  - [x] テスト用UI（ボタン等）の作成
  - [x] 動作確認

## Phase 2: Dashboard UI基盤の構築
- [x] **Frontend:** `grid-layout-plus` の導入
- [x] **Frontend:** ダッシュボードコンポーネントの作成
  - [x] 閲覧モード/編集モードのステート管理
  - [x] グリッドレイアウトの実装 (`DashboardGrid.vue`)
- [x] **Frontend:** ウィジェット管理システム
  - [x] `WidgetHost.vue` (動的コンポーネントローダー) の作成
  - [x] ウィジェット設定の型定義 (`WidgetConfig` 等)

## Phase 3: 基本ウィジェットの実装
- [x] **Frontend:** 数値表示ウィジェット (Value Display)
- [x] **Frontend:** スイッチ/ボタンウィジェット (Publish機能連携)
- [x] **Frontend:** 折れ線グラフウィジェット (Line Chart)
  - [x] `vue-echarts` / `echarts` の導入
  - [x] リアルタイム更新ロジックの実装
- [x] **Frontend:** ゲージウィジェット (Gauge)

## Phase 4: データ履歴と設定管理
- [x] **Backend:** 履歴データ取得コマンド (`get_history`) の実装
  - [x] SQLiteからの範囲検索と間引き処理
  - [x] DB接続エラーハンドリングの強化 (アプリクラッシュ回避)
  - [x] `tauri-plugin-sql` の廃止と `sqlx` によるテーブル管理への移行
- [x] **Frontend:** グラフへの履歴データ反映
- [x] **System:** レイアウト設定の保存・読み込み
  - [x] Backend: ファイルダイアログを使用した任意のパスへの保存・読み込み
  - [x] Backend: 前回使用したレイアウトパスの記憶 (`config.json`)
  - [x] Frontend: UIへの保存・開くボタンの追加とダイアログ連携

## Phase 5: アプリ設定と最適化
- [ ] **Frontend:** アプリ全体設定モーダル
  - [ ] 接続設定 (Broker Host/Port)
  - [ ] テーマ切り替え
- [ ] **Backend:** 接続設定の動的反映 (Internal/External切り替え対応)
- [ ] **Backend:** 古いデータの自動削除 (Retention Policy)
