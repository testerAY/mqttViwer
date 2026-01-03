# 機能拡張開発 ToDoリスト (ウィジェット機能強化)

本ドキュメントは、ウィジェット機能の大幅な拡張に関する作業項目をまとめたものである。

## 1. 共通基盤・データ構造の改修 (Core & Types)

既存のデータ構造を見直し、複数データソース対応および更新頻度制御の基盤を作成する。

- [x] **型定義の更新 (`src/types/dashboard.ts`)**
    - `WidgetConfig` インターフェースに `updateInterval` (更新頻度/ms) フィールドを追加する（Optional, Default: 0）。
    - `WidgetConfig` の `settings` 内に、複数データ系列を管理するための `series` 配列定義を追加する。
- [x] **CSVエクスポート制御のロジック追加 (`src/components/dashboard/WidgetHost.vue`)**
    - 現在実装されている「CSV Export」ボタンに対し、ウィジェットタイプに基づく表示制御(`v-if`)を追加する。
    - **非表示対象:** `value-display`, `gauge`, `switch`, `slider` (単一現在値のみのウィジェット)。
    - **表示対象:** `chart` (Line/Bar), `plotter`, `scatter`, `gantt` (時系列または複数データを持つウィジェット)。

## 2. 画面更新頻度設定の実装 (Update Frequency)

パフォーマンス制御のため、ウィジェットごとの再描画頻度を設定可能にする。

- [x] **設定モーダルの改修 (`src/components/dashboard/WidgetSettingsModal.vue`)**
    - 「General」タブに「Update Frequency (ms)」入力欄を追加する。
    - 0の場合はリアルタイム（メッセージ受信即反映）、数値がある場合はその間隔でThrottle処理を行う。
- [x] **更新制御の実装 (`DashboardGrid.vue` or Widget Wrapper)**
    - WidgetコンポーネントへのProps伝達、または各コンポーネント内の `watch` 処理において、設定された `updateInterval` を考慮して描画更新をスキップ/遅延させるロジックを実装する。

## 3. 複数値（Multi-Series）表示とチャート機能強化

`ChartWidget` を単一トピック・単一キー依存から、複数系列対応へアップグレードする。

- [x] **設定モーダル: Data Mappingタブの刷新 (`WidgetSettingsModal.vue`)**
    - 既存の単一 `Value Key` 入力欄を、後方互換用として残すか、自動的に「Series 1」として扱うようにUIを変更する。
    - **Series List UIの実装:**
        - シリーズの追加・削除・並べ替え機能。
        - 各シリーズ設定: `Topic` (Global選択可), `Value Key`, `Label Name`, `Line/Bar Color`。
- [x] **設定モーダル: Styleタブの拡張**
    - 「Chart Type」選択肢を追加 (`Line` / `Bar`)。
- [x] **ChartWidgetのロジック改修 (`src/components/widgets/ChartWidget.vue`)**
    - `props.config.settings.series` を読み込み、EChartsの `series` オプションを動的に生成するループ処理を実装する。
    - データ受信時、トピックごとに該当するシリーズのデータを更新するロジックに変更する。
    - `chartType` 設定に基づき、EChartsの `type` を `line` または `bar` に切り替える。
    - 凡例 (Legend) を有効化し、複数系列の識別を可能にする。

## 4. 新規ウィジェットの追加

新しい可視化コンポーネントを作成し、`widgetRegistry` に登録する。

### 4.1 Plotter (Strip Chart)
リアルタイムな信号監視に特化した、流れるようなチャート。
- [x] **コンポーネント作成 (`src/components/widgets/PlotterWidget.vue`)**
    - X軸を「現在時刻」ではなく「相対時間（例: -60s ～ 0s）」で固定表示するモード。
    - 古いデータは画面外に出た瞬間にメモリから破棄する（リングバッファ的な挙動）。
- [x] **レジストリ登録**

### 4.2 Time Gantt Chart (Status History)
状態遷移（ON/OFF、ステータス文字列など）を時系列の帯として表示する。
- [x] **コンポーネント作成 (`src/components/widgets/GanttWidget.vue`)**
    - EChartsの `custom` レンダラ、または積み上げ横棒グラフを応用して実装。
    - Y軸: トピック名また機器名。
    - X軸: 時間経過。
    - 値と色のマッピング設定（例: "Running"->緑, "Stop"->赤）を設定画面に追加する必要がある。(Basic implementation done)
- [x] **レジストリ登録**

### 4.3 Scatter Plot (Correlation)
2つの異なるセンサー値の相関を確認するための散布図。
- [x] **コンポーネント作成 (`src/components/widgets/ScatterWidget.vue`)**
    - 設定画面で「X軸のTopic/Key」と「Y軸のTopic/Key」を個別に指定可能にする。
    - 時間軸ではなく、値 vs 値 のプロットを行う。
- [x] **レジストリ登録**

## 5. マイグレーションとテスト

- [ ] **データ互換性チェック**
    - 既存のダッシュボード設定ファイル (`app_data.db` 内) を読み込んだ際、旧形式の `ChartWidget` がエラーにならず表示されること。
    - 旧設定（`valueKey`）が存在する場合、自動的に `series[0]` として解釈されるロジックが含まれているか確認。
- [ ] **負荷テスト**
    - 複数シリーズ（例: 5本）を表示したチャートに対し、高頻度（10ms〜50ms）でメッセージを送出し、ブラウザのメモリ使用量とFPSを確認する。
