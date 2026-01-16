# Elm（エルム）Ports（JavaScript 連携）

## 概要

Ports は Elm と JavaScript の間でデータをやり取りする仕組み。
Elm の純粋性を保ちながら、外部の副作用を扱える。

## 基本構造

```elm
-- src/Ports.elm
port module Ports exposing (sendMessage, receiveMessage)

import Json.Encode as Encode

-- Elm → JavaScript
port sendMessage : Encode.Value -> Cmd msg

-- JavaScript → Elm
port receiveMessage : (Encode.Value -> msg) -> Sub msg
```

```javascript
// js/ports/index.js
app.ports.sendMessage.subscribe(function(data) {
    // Elm から受信
    console.log('Received from Elm:', data);
});

// Elm に送信
app.ports.receiveMessage.send({ type: 'CONNECTED' });
```

## メッセージエンベロープ

Ports を介したメッセージは標準フォーマットを使用する。

```json
{
  "v": 1,
  "type": "WORKFLOW_UPDATED",
  "payload": { ... },
  "correlationId": "550e8400-e29b-41d4-a716-446655440000",
  "ts": 1705142400000
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `v` | Int | メッセージバージョン（後方互換性のため） |
| `type` | String | メッセージタイプ |
| `payload` | Value | ペイロード |
| `correlationId` | String | 追跡用 ID |
| `ts` | Int | タイムスタンプ（Unix ms） |

## 受信データの検証

JavaScript からのデータは型安全ではない。必ず `Json.Decode` で検証する。

```elm
type alias Message =
    { version : Int
    , messageType : String
    , payload : Decode.Value
    , correlationId : String
    , timestamp : Int
    }

messageDecoder : Decode.Decoder Message
messageDecoder =
    Decode.map5 Message
        (Decode.field "v" Decode.int)
        (Decode.field "type" Decode.string)
        (Decode.field "payload" Decode.value)
        (Decode.field "correlationId" Decode.string)
        (Decode.field "ts" Decode.int)

-- 受信時の処理
handleReceive : Decode.Value -> Msg
handleReceive value =
    case Decode.decodeValue messageDecoder value of
        Ok message ->
            ReceivedMessage message

        Err error ->
            ReceivedInvalidMessage (Decode.errorToString error)
```

## Elm 側の update 関数

```elm
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        ReceivedMessage message ->
            case message.messageType of
                "WORKFLOW_UPDATED" ->
                    handleWorkflowUpdated message.payload model

                "CONNECTION_LOST" ->
                    ( { model | connected = False }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        ReceivedInvalidMessage error ->
            -- ログに記録、UI にはエラー表示しない
            ( model, logError error )
```

## JavaScript 側の実装

```javascript
// js/ports/index.js
export function setupPorts(app) {
    // WebSocket 接続
    const ws = new WebSocket('wss://api.example.com/ws');

    // サーバーからのメッセージを Elm に転送
    ws.onmessage = (event) => {
        const data = JSON.parse(event.data);
        app.ports.receiveMessage.send({
            v: 1,
            type: data.type,
            payload: data.payload,
            correlationId: data.correlationId || crypto.randomUUID(),
            ts: Date.now()
        });
    };

    // Elm からのメッセージをサーバーに送信
    app.ports.sendMessage.subscribe((data) => {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify(data));
        }
    });

    // 接続状態の通知
    ws.onopen = () => {
        app.ports.receiveMessage.send({
            v: 1,
            type: 'CONNECTED',
            payload: null,
            correlationId: crypto.randomUUID(),
            ts: Date.now()
        });
    };

    ws.onclose = () => {
        app.ports.receiveMessage.send({
            v: 1,
            type: 'DISCONNECTED',
            payload: null,
            correlationId: crypto.randomUUID(),
            ts: Date.now()
        });
        reconnect();
    };
}
```

## WebSocket 再接続

指数バックオフ + ジッタで再接続する。

```javascript
let reconnectAttempt = 0;

function reconnect() {
    const baseDelay = 1000;    // 1秒
    const maxDelay = 30000;    // 30秒
    const jitter = Math.random() * 1000;  // 0-1秒のランダム

    const delay = Math.min(
        baseDelay * Math.pow(2, reconnectAttempt),
        maxDelay
    ) + jitter;

    setTimeout(() => {
        reconnectAttempt++;
        connect();
    }, delay);
}

function onConnected() {
    reconnectAttempt = 0;  // リセット
}
```

### なぜ指数バックオフ + ジッタか

- 指数バックオフ: サーバー障害時に全クライアントが同時に再接続するのを防ぐ
- ジッタ: さらに分散させて、再接続の「雪崩」を防ぐ

## 状態再同期

再接続時やシーケンスギャップ検出時は、HTTP 経由で状態を再同期する。

```elm
type Msg
    = WebSocketConnected
    | GotSyncData (Result Http.Error SyncData)

update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        WebSocketConnected ->
            -- 再接続したら状態を同期
            ( model, fetchSyncData )

        GotSyncData (Ok data) ->
            ( applySync data model, Cmd.none )
```

## Ports モジュールの集約

プロジェクト内の全 Ports は 1 つのモジュールに集約する。

```elm
-- src/Ports.elm
port module Ports exposing
    ( sendMessage
    , receiveMessage
    , saveToLocalStorage
    , loadFromLocalStorage
    , localStorageLoaded
    )

-- WebSocket
port sendMessage : Encode.Value -> Cmd msg
port receiveMessage : (Encode.Value -> msg) -> Sub msg

-- LocalStorage
port saveToLocalStorage : { key : String, value : Encode.Value } -> Cmd msg
port loadFromLocalStorage : String -> Cmd msg
port localStorageLoaded : ({ key : String, value : Encode.Value } -> msg) -> Sub msg
```

## プロジェクトでの使用

| ファイル | 役割 |
|---------|------|
| `apps/web/src/Ports.elm` | Ports 定義 |
| `apps/web/js/ports/index.js` | JavaScript 側の実装 |

## 関連リソース

- [Elm 公式ガイド - Ports](https://guide.elm-lang.org/interop/ports.html)
- [Elm in Action - Chapter 7](https://www.manning.com/books/elm-in-action)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-14 | 初版作成 |
