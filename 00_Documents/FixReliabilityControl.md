# 修正タスクリスト (優先度順)

産業用アプリケーションとしての「信頼性」と「操作性」を確保するための修正タスクリストです。

### [完了] 優先度1: Backend - DB書き込みのバッチ化 (Critical)
**目的:** 高負荷時のアプリフリーズとDBロック回避

* **対象ファイル:** `src-tauri/src/mqtt.rs`
* **現状:** 1メッセージ受信ごとに `INSERT` を実行している。
* **実装内容:**
    1.  受信ループ内にバッファ（`Vec<MqttMessage>`）を作成。
    2.  「100件到達」または「500ms経過」でバッファ内のデータをDBへ一括書き込み（Bulk Insertまたはトランザクション）するロジックへ変更。

### [完了] 優先度2: Backend - 購読トピックのフィルタリング設定 (Risk Management)
**目的:** 不要なパケット受信によるリソース浪費とDB汚染の防止

* **対象ファイル:** `src-tauri/src/mqtt.rs`, `src-tauri/src/config.rs`（設定定義）, `src/components/SettingsModal.vue`
* **現状:** `client_clone.subscribe("#", ...)` とハードコードされており、ブローカー上の全メッセージを受信してしまう。
* **実装内容:**
    1.  **Backend:** `AppConfig` 構造体（`config.rs`）に `subscription_topic` フィールドを追加（デフォルト値: `"#"`）。
    2.  **Backend:** `mqtt.rs` の `subscribe` 呼び出し部分を、設定値を使用するように変更する。
        ```rust
        // 変更イメージ
        let topic = &config.broker.subscription_topic; // 設定から取得
        client_clone.subscribe(topic, QoS::AtMostOnce).await
        ```
    3.  **Frontend:** アプリ全体設定モーダル（`SettingsModal.vue`）に「Subscription Topic」入力欄を追加し、ルートトピック（例: `factory/line1/#`）を指定できるようにする。

### [完了] 優先度3: Frontend - スイッチ操作のフィードバック実装 (UX)
**目的:** 操作反応待ち時の連打・誤認防止

* **対象ファイル:** `src/components/widgets/SwitchWidget.vue`
* **現状:** Publish処理中、UIに変化がない。
* **実装内容:**
    1.  `isPublishing` フラグ（ref）を追加。
    2.  Publish開始～終了までフラグを `true` にし、その間スイッチを `disabled` 状態かつ半透明（`opacity-50`）にする。

### [完了] 優先度4: Frontend - エラー通知（Toast）の導入 (Reliability)
**目的:** エラー発生の可視化

* **対象:** 新規コンポーネント / ストア
* **現状:** エラーが `console.error` にしか出力されない。
* **実装内容:**
    1.  `useToastStore` と `ToastContainer` コンポーネントを作成。
    2.  Publish失敗時やDBエラー時にトースト通知を表示するよう連携させる。

### [完了] 優先度5: Frontend - 設定画面の入力支援機能 (Usability)
**目的:** 設定ミスの低減

* **対象ファイル:** `src/components/dashboard/WidgetSettingsModal.vue`
* **現状:** トピックやキー指定が完全手入力。
* **実装内容:**
    1.  Backendから「受信済みトピック一覧」を取得するコマンドを実装し、設定画面でコンボボックス選択できるようにする。
    2.  設定画面内で「現在のRaw JSON」と「抽出結果」をプレビュー表示し、キー指定（`data.temp` 等）が正しいか即座に確認できるようにする。
