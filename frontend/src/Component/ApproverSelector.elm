module Component.ApproverSelector exposing
    ( ApproverSelection(..)
    , KeyResult(..)
    , State
    , handleKeyDown
    , init
    , selectedUserId
    , view
    )

{-| 承認者選択コンポーネント

検索可能なドロップダウンで承認者を選択する UI コンポーネント。
キーボードナビゲーション（ArrowDown/Up、Enter、Escape）をサポートする。

型変数 `msg` により、各ページの `Msg` 型に対応。


## 使用例

    import Component.ApproverSelector as ApproverSelector

    -- Model に State を含める
    type alias Model =
        { approver : ApproverSelector.State
        , ...
        }

    -- view でコンポーネントを描画
    ApproverSelector.view
        { state = model.approver
        , users = model.users
        , validationError = Dict.get "approver" model.validationErrors
        , onSearch = UpdateApproverSearch
        , onSelect = SelectApprover
        , onClear = ClearApprover
        , onKeyDown = ApproverKeyDown
        , onCloseDropdown = CloseApproverDropdown
        }

-}

import Api exposing (ApiError)
import Data.UserItem as UserItem exposing (UserItem)
import Data.UserRef exposing (UserRef)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Json.Decode as Decode
import List.Extra
import RemoteData exposing (RemoteData(..))



-- 型定義


{-| 承認者の選択状態

`Preselected` はワークフローの既存データから事前設定された状態（id + name のみ）。
`Selected` はドロップダウンから明示的に選択された状態（全ユーザー情報あり）。

-}
type ApproverSelection
    = NotSelected
    | Preselected UserRef
    | Selected UserItem


{-| コンポーネントの状態

親ページの Model に含めて管理する。

-}
type alias State =
    { selection : ApproverSelection
    , search : String
    , dropdownOpen : Bool
    , highlightIndex : Int
    }


{-| 初期状態
-}
init : State
init =
    { selection = NotSelected
    , search = ""
    , dropdownOpen = False
    , highlightIndex = 0
    }


{-| 選択状態からユーザー ID を取得

NotSelected の場合は Nothing を返す。
Preselected / Selected のいずれでも ID を返す。

-}
selectedUserId : ApproverSelection -> Maybe String
selectedUserId selection =
    case selection of
        NotSelected ->
            Nothing

        Preselected ref ->
            Just ref.id

        Selected user ->
            Just user.id


{-| キーボード操作の結果

`handleKeyDown` の戻り値。親ページが pattern match して
副作用（dirty 状態更新、バリデーションエラー解除等）を処理する。

-}
type KeyResult
    = NoChange
    | Navigate Int
    | Select UserItem
    | Close



-- ロジック


{-| キーボードイベントを処理

純粋関数として、キー入力と候補リストから操作結果を返す。
候補のフィルタリングは呼び出し元が行い、結果を `candidates` として渡す。

-}
handleKeyDown :
    { key : String
    , candidates : List UserItem
    , highlightIndex : Int
    }
    -> KeyResult
handleKeyDown { key, candidates, highlightIndex } =
    let
        candidateCount =
            List.length candidates
    in
    case key of
        "ArrowDown" ->
            if candidateCount == 0 then
                NoChange

            else
                Navigate (modBy candidateCount (highlightIndex + 1))

        "ArrowUp" ->
            if candidateCount == 0 then
                NoChange

            else
                Navigate (modBy candidateCount (highlightIndex - 1 + candidateCount))

        "Enter" ->
            case List.Extra.getAt highlightIndex candidates of
                Just user ->
                    Select user

                Nothing ->
                    NoChange

        "Escape" ->
            Close

        _ ->
            NoChange



-- VIEW


{-| 承認者選択 UI を描画

選択状態に応じて「選択済み表示」または「検索入力 + ドロップダウン」を表示する。
バリデーションエラーがある場合はエラーメッセージも表示する。

-}
view :
    { state : State
    , users : RemoteData ApiError (List UserItem)
    , validationError : Maybe String
    , onSearch : String -> msg
    , onSelect : UserItem -> msg
    , onClear : msg
    , onKeyDown : String -> msg
    , onCloseDropdown : msg
    }
    -> Html msg
view config =
    div []
        [ case config.state.selection of
            Selected user ->
                viewSelectedApprover user.name (Just user.displayId) config.onClear

            Preselected ref ->
                viewSelectedApprover ref.name Nothing config.onClear

            NotSelected ->
                viewSearchInput config
        , viewError config.validationError
        ]


{-| 選択済みの承認者を表示

name は常に表示。displayId は Preselected 状態では Nothing になるため、
値がある場合のみ表示する。

-}
viewSelectedApprover : String -> Maybe String -> msg -> Html msg
viewSelectedApprover name maybeDisplayId onClear =
    div
        [ class "flex items-center justify-between rounded-lg border border-primary-200 bg-primary-50 p-3" ]
        [ div []
            [ span [ class "font-medium" ] [ text name ]
            , case maybeDisplayId of
                Just displayId ->
                    span [ class "ml-2 text-sm text-secondary-500" ] [ text displayId ]

                Nothing ->
                    text ""
            ]
        , button
            [ Html.Events.onClick onClear
            , class "border-0 bg-transparent cursor-pointer text-secondary-400 hover:text-secondary-600 transition-colors text-xl rounded outline-none focus-visible:ring-2 focus-visible:ring-primary-500"
            , type_ "button"
            , attribute "aria-label" "承認者を解除"
            ]
            [ text "×" ]
        ]


{-| 承認者検索入力とドロップダウン
-}
viewSearchInput :
    { a
        | state : State
        , users : RemoteData ApiError (List UserItem)
        , onSearch : String -> msg
        , onSelect : UserItem -> msg
        , onKeyDown : String -> msg
        , onCloseDropdown : msg
    }
    -> Html msg
viewSearchInput config =
    let
        candidates =
            case config.users of
                Success users ->
                    UserItem.filterUsers config.state.search users

                _ ->
                    []
    in
    div [ class "relative" ]
        [ input
            [ type_ "text"
            , id "approver-search"
            , attribute "aria-label" "承認者を検索"
            , Html.Attributes.value config.state.search
            , Html.Events.onInput config.onSearch
            , Html.Events.onBlur config.onCloseDropdown
            , Html.Events.preventDefaultOn "keydown"
                (Decode.field "key" Decode.string
                    |> Decode.map
                        (\key ->
                            ( config.onKeyDown key
                            , key == "ArrowDown" || key == "ArrowUp"
                            )
                        )
                )
            , placeholder "名前で検索..."
            , Html.Attributes.autocomplete False
            , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            ]
            []
        , if config.state.dropdownOpen && not (List.isEmpty candidates) then
            viewDropdown candidates config.state.highlightIndex config.onSelect

          else if config.state.dropdownOpen && not (String.isEmpty (String.trim config.state.search)) then
            viewNoResults

          else
            text ""
        , case config.users of
            Loading ->
                p [ class "mt-2 text-sm text-secondary-500" ] [ text "ユーザー情報を読み込み中..." ]

            Failure _ ->
                p [ class "mt-2 text-sm text-error-600" ] [ text "ユーザー情報の取得に失敗しました" ]

            _ ->
                text ""
        ]


{-| 候補ドロップダウン
-}
viewDropdown : List UserItem -> Int -> (UserItem -> msg) -> Html msg
viewDropdown candidates highlightIndex onSelect =
    ul
        [ class "absolute z-10 mt-1 w-full rounded-lg border border-secondary-200 bg-white shadow-lg max-h-60 overflow-y-auto"
        ]
        (List.indexedMap (viewCandidate highlightIndex onSelect) candidates)


{-| 候補アイテム
-}
viewCandidate : Int -> (UserItem -> msg) -> Int -> UserItem -> Html msg
viewCandidate highlightIndex onSelect index user =
    li
        [ Html.Events.onMouseDown (onSelect user)
        , class
            ("px-3 py-2 cursor-pointer"
                ++ (if index == highlightIndex then
                        " bg-primary-50"

                    else
                        " hover:bg-primary-50 transition-colors"
                   )
            )
        ]
        [ div [ class "font-medium" ] [ text user.name ]
        , div [ class "text-sm text-secondary-500" ]
            [ text (user.displayId ++ " · " ++ user.email) ]
        ]


{-| 候補なし表示
-}
viewNoResults : Html msg
viewNoResults =
    div
        [ class "absolute z-10 mt-1 w-full rounded-lg border border-secondary-200 bg-white shadow-lg px-3 py-2 text-sm text-secondary-500" ]
        [ text "該当するユーザーが見つかりません" ]


{-| バリデーションエラー表示
-}
viewError : Maybe String -> Html msg
viewError maybeErrorMsg =
    case maybeErrorMsg of
        Just errorMsg ->
            div
                [ class "mt-1 text-sm text-error-600" ]
                [ text errorMsg ]

        Nothing ->
            text ""
