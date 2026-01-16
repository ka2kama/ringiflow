module ReviewConfig exposing (config)

{-| elm-review 設定
-}

import NoUnused.CustomTypeConstructorArgs
import NoUnused.CustomTypeConstructors
import NoUnused.Dependencies
import NoUnused.Exports
import NoUnused.Modules
import NoUnused.Parameters
import NoUnused.Patterns
import NoUnused.Variables
import Review.Rule as Rule exposing (Rule)
import Simplify


config : List Rule
config =
    [ -- 未使用の変数・関数
      NoUnused.Variables.rule
    , -- 未使用のカスタム型コンストラクタ
      NoUnused.CustomTypeConstructors.rule []
    , -- 未使用のコンストラクタ引数
      NoUnused.CustomTypeConstructorArgs.rule
    , -- 未使用の依存関係
      NoUnused.Dependencies.rule
    , -- 未使用のエクスポート
      NoUnused.Exports.rule
        -- TODO: Ports.elm は BFF 連携実装時に使用予定。使用開始後にこの除外設定を削除する
        |> Rule.ignoreErrorsForFiles [ "src/Ports.elm" ]
    , -- 未使用のモジュール
      NoUnused.Modules.rule
        -- TODO: Ports.elm は BFF 連携実装時に使用予定。使用開始後にこの除外設定を削除する
        |> Rule.ignoreErrorsForFiles [ "src/Ports.elm" ]
    , -- 未使用のパラメータ
      NoUnused.Parameters.rule
    , -- 未使用のパターン
      NoUnused.Patterns.rule
    , -- コード簡略化
      Simplify.rule Simplify.defaults
    ]
