port module Ports exposing
    ( receiveMessage
    , sendMessage
    )

{-| JavaScript との通信用 Ports モジュール

Elm は純粋関数型言語であり、直接 JavaScript を呼び出せない。
Ports は Elm と JavaScript の間に型安全な通信チャネルを提供する。


## Ports の設計思想

Elm の「ランタイムエラーなし」保証を維持するため、
Ports は以下の制約を持つ:

1.  **一方向通信**: 送信と受信は別々の Port
2.  **JSON 経由**: データは JSON.Encode.Value として送受信
3.  **エラー非伝播**: JavaScript 側のエラーは Elm に伝わらない

これにより、JavaScript の予測不能な挙動から Elm を保護する。


## データフロー

```text
Elm → JavaScript (送信):
    Elm Cmd → sendMessage port → JavaScript subscribe

JavaScript → Elm (受信):
    JavaScript send → receiveMessage port → Elm Sub → Msg
```


## メッセージフォーマット

本プロジェクトでは以下の構造化メッセージを推奨:

```json
{
    "v": 1,
    "type": "SOME_ACTION",
    "payload": { ... },
    "correlationId": "uuid-string",
    "ts": 1234567890
}
```

  - `v`: スキーマバージョン（互換性管理）
  - `type`: メッセージの種類（アクション名）
  - `payload`: 実際のデータ
  - `correlationId`: リクエスト-レスポンスの紐付け
  - `ts`: タイムスタンプ


## 代替案と不採用理由

  - **Web Components**: Elm 内での使用は複雑
  - **Custom Elements**: Ports より設定が煩雑
  - **elm-typescript-interop**: 追加の依存関係が必要


## セキュリティ考慮

Ports 経由のデータは信頼できない外部入力として扱う。
受信時は必ずデコーダーで検証すること。

-}

import Json.Encode as Encode


{-| JavaScript へメッセージを送信


## 使用方法

Elm 側:

    sendMessageCmd : String -> Cmd msg
    sendMessageCmd content =
        sendMessage
            (Encode.object
                [ ( "v", Encode.int 1 )
                , ( "type", Encode.string "NOTIFY" )
                , ( "payload"
                  , Encode.object
                        [ ( "content", Encode.string content )
                        ]
                  )
                ]
            )

JavaScript 側:

```javascript
app.ports.sendMessage.subscribe((data) => {
    console.log("Received from Elm:", data);
    // data.type に基づいて処理を分岐
    switch (data.type) {
        case "NOTIFY":
            showNotification(data.payload.content);
            break;
        // ...
    }
});
```


## 型シグネチャの解説

`Encode.Value -> Cmd msg`

  - `Encode.Value`: 任意の JSON 値（型安全に構築）
  - `Cmd msg`: 副作用を表すコマンド
  - `msg`: 型パラメータ（この Port は Msg を発生させない）

-}
port sendMessage : Encode.Value -> Cmd msg


{-| JavaScript からメッセージを受信


## 使用方法

JavaScript 側:

```javascript
// Elm へメッセージを送信
app.ports.receiveMessage.send({
    v: 1,
    type: "USER_LOGGED_IN",
    payload: { userId: "123", name: "Alice" },
    correlationId: crypto.randomUUID(),
    ts: Date.now()
});
```

Elm 側:

    -- Msg に追加
    type Msg
        = ...
        | ReceivedMessage Encode.Value


    -- subscriptions で購読
    subscriptions model =
        receiveMessage ReceivedMessage


    -- update で処理
    update msg model =
        case msg of
            ReceivedMessage value ->
                case Decode.decodeValue messageDecoder value of
                    Ok message ->
                        handleMessage message model

                    Err _ ->
                        -- 不正なメッセージは無視
                        ( model, Cmd.none )


## 型シグネチャの解説

`(Encode.Value -> msg) -> Sub msg`

  - `(Encode.Value -> msg)`: 受信データを Msg に変換する関数
  - `Sub msg`: 購読（外部イベントを Msg に変換）

これは高階関数パターンで、「どの Msg に変換するか」を
呼び出し側が決定できる。

-}
port receiveMessage : (Encode.Value -> msg) -> Sub msg
