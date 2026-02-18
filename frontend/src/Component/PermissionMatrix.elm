module Component.PermissionMatrix exposing (view)

{-| 権限マトリクスコンポーネント

リソース × アクションのチェックボックスグリッド。
各リソース行に「すべて選択」トグルを提供する。

権限は `"resource:action"` 形式の文字列で管理される。

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onCheck)
import Set exposing (Set)



-- CONFIG


{-| 権限マトリクスの設定

  - `selectedPermissions`: 選択中の権限セット
  - `onToggle`: 個別権限のトグル（例: `"workflow:read"`）
  - `onToggleAll`: リソース全体のトグル（例: `"workflow"`）
  - `disabled`: 全体を無効化（システムロールの読み取り専用表示用）

-}
type alias Config msg =
    { selectedPermissions : Set String
    , onToggle : String -> msg
    , onToggleAll : String -> msg
    , disabled : Bool
    }


{-| リソース定義
-}
resources : List ( String, String )
resources =
    [ ( "workflow", "ワークフロー" )
    , ( "task", "タスク" )
    ]


{-| アクション定義
-}
actions : List ( String, String )
actions =
    [ ( "read", "閲覧" )
    , ( "create", "作成" )
    , ( "update", "更新" )
    , ( "delete", "削除" )
    ]



-- VIEW


view : Config msg -> Html msg
view config =
    div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    (th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ]
                        [ text "リソース" ]
                        :: th [ class "px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-secondary-600" ]
                            [ text "すべて" ]
                        :: List.map
                            (\( _, actionLabel ) ->
                                th [ class "px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-secondary-600" ]
                                    [ text actionLabel ]
                            )
                            actions
                    )
                ]
            , tbody [ class "divide-y divide-secondary-200 bg-white" ]
                (List.map (viewResourceRow config) resources)
            ]
        ]


viewResourceRow : Config msg -> ( String, String ) -> Html msg
viewResourceRow config ( resourceKey, resourceLabel ) =
    let
        allPermissions =
            List.map (\( actionKey, _ ) -> resourceKey ++ ":" ++ actionKey) actions

        allSelected =
            List.all (\p -> Set.member p config.selectedPermissions) allPermissions
    in
    tr [ class "hover:bg-secondary-50 transition-colors" ]
        (td [ class "px-4 py-3 text-sm font-medium text-secondary-900" ]
            [ text resourceLabel ]
            :: td [ class "px-4 py-3 text-center" ]
                [ input
                    [ type_ "checkbox"
                    , checked allSelected
                    , onCheck (\_ -> config.onToggleAll resourceKey)
                    , disabled config.disabled
                    , class "h-4 w-4 rounded border-secondary-300 text-primary-600 outline-none focus-visible:ring-2 focus-visible:ring-primary-500"
                    ]
                    []
                ]
            :: List.map
                (\( actionKey, _ ) ->
                    let
                        permission =
                            resourceKey ++ ":" ++ actionKey

                        isSelected =
                            Set.member permission config.selectedPermissions
                    in
                    td [ class "px-4 py-3 text-center" ]
                        [ input
                            [ type_ "checkbox"
                            , checked isSelected
                            , onCheck (\_ -> config.onToggle permission)
                            , disabled config.disabled
                            , class "h-4 w-4 rounded border-secondary-300 text-primary-600 outline-none focus-visible:ring-2 focus-visible:ring-primary-500"
                            ]
                            []
                        ]
                )
                actions
        )
