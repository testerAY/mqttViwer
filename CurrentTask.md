# Data Mapping 一元化機能 実装ToDoリスト

## Phase 1: データ構造とストアの拡張
Data Mappingの情報を管理するための基盤を作成します。

- [ ] **型定義の追加 (`src/types/dashboard.ts`)**
    - [ ] `DataMapping` インターフェースを定義する。
        - フィールド: `id`, `name`, `type` ('sub' | 'pub'), `topic`, `valueKey`, `description?`
    - [ ] `DashboardLayout` 型（またはそれをラップする親の型）に `dataMappings: DataMapping[]` フィールドを追加。

- [ ] **ストアの更新 (`src/stores/useDashboardStore.ts`)**
    - [ ] `dataMappings` state変数を追加。
    - [ ] アクションの実装:
        - `addDataMapping(mapping: DataMapping)`
        - `updateDataMapping(id: string, mapping: DataMapping)`
        - `removeDataMapping(id: string)`
        - `getDataMappingById(id: string)`
    - [ ] `saveLayout` / `loadLayout` ロジックの修正:
        - `items` だけでなく `dataMappings` も保存・復元対象にする。

## Phase 2: Data Mapping 設定UIの実装
ユーザーがデータ定義を行うための管理画面を作成します。

- [ ] **新規コンポーネント作成 (`src/components/dashboard/DataMappingModal.vue`)**
    - [ ] モーダルウィンドウのUI作成（左側リスト、右側詳細）。
    - [ ] 詳細編集フォームの実装:
        - Data Type (Subscribe / Publish)
        - Name (識別名)
        - Topic (入力/補完)
        - Value Key (JSONパス)
    - [ ] プレビュー機能の実装（既存ロジック流用）。

- [ ] **ヘッダーへの導線追加 (`src/App.vue`)**
    - [ ] 「Data Mappings」ボタンを追加。
    - [ ] モーダル開閉制御の実装。

## Phase 3: ウィジェット設定画面の改修
ウィジェット設定でTopicを直接入力するのではなく、Mappingを選択する形式に変更します。

- [ ] **設定モーダルの修正 (`src/components/dashboard/WidgetSettingsModal.vue`)**
    - [ ] **Generalタブ**:
        - `MQTT Topic` 入力を削除し、`Data Source` セレクトボックスに変更。
        - マッピングの `name` をリスト表示。
    - [ ] **Data Mappingタブ**:
        - `Value Key` 入力を削除。
        - チャート等のSeries設定でもMapping選択を使用するように変更。
    - [ ] **データ保存構造の変更**:
        - `topic` / `valueKey` の代わりに `mappingId` を保存。

## Phase 4: ウィジェット本体のデータ取得ロジック修正
設定された `mappingId` を元にデータを取得・送信するように変更します。

- [ ] **データ解決ロジックの実装**
    - [ ] Composable `useWidgetData(widgetId)` の作成。
        - `mappingId` から `topic`, `key` を解決し、現在の値を返す。

- [ ] **各ウィジェットの修正**
    - [ ] `ValueDisplayWidget`, `ChartWidget` 等: `topic` プロパティの代わりに解決されたデータを使用。
    - [ ] `SwitchWidget`, `SliderWidget` (Publish系): Publish先をMappingのTopicに変更。

## (オプション) 移行・互換性対応
- [ ] **マイグレーション処理**
    - [ ] 旧形式のレイアウト読み込み時、自動的にData Mapping定義を生成してリストに追加する処理の実装。