module Page.Workflow.Detail.Comments exposing (updateComments, viewCommentSection)

{-| コメントセクション

ワークフローに紐づくコメントスレッドの表示・投稿を管理する。

-}

import Api.Workflow as WorkflowApi
import Component.Button as Button
import Component.LoadingSpinner as LoadingSpinner
import Data.WorkflowComment exposing (WorkflowComment)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Page.Workflow.Detail.Types exposing (LoadedState, Msg(..))
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)



-- UPDATE


{-| コメント関連メッセージの処理
-}
updateComments : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateComments msg shared workflowDisplayNumber loaded =
    case msg of
        GotComments result ->
            case result of
                Ok comments ->
                    ( { loaded | comments = Success comments }, Cmd.none )

                Err err ->
                    ( { loaded | comments = Failure err }, Cmd.none )

        UpdateNewComment body ->
            ( { loaded | newCommentBody = body }, Cmd.none )

        SubmitComment ->
            if String.isEmpty (String.trim loaded.newCommentBody) then
                ( loaded, Cmd.none )

            else
                ( { loaded | isPostingComment = True }
                , WorkflowApi.postComment
                    { config = Shared.toRequestConfig shared
                    , displayNumber = workflowDisplayNumber
                    , body = { body = String.trim loaded.newCommentBody }
                    , toMsg = GotPostCommentResult
                    }
                )

        GotPostCommentResult result ->
            case result of
                Ok newComment ->
                    let
                        updatedComments =
                            case loaded.comments of
                                Success existing ->
                                    Success (existing ++ [ newComment ])

                                _ ->
                                    Success [ newComment ]
                    in
                    ( { loaded
                        | comments = updatedComments
                        , newCommentBody = ""
                        , isPostingComment = False
                      }
                    , Cmd.none
                    )

                Err _ ->
                    ( { loaded
                        | isPostingComment = False
                        , errorMessage = Just "コメントの投稿に失敗しました。"
                      }
                    , Cmd.none
                    )

        _ ->
            ( loaded, Cmd.none )



-- VIEW


{-| コメントセクション

ワークフローに紐づくコメントスレッドを表示し、新規コメントの投稿を提供する。

-}
viewCommentSection : LoadedState -> Html Msg
viewCommentSection loaded =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "コメント" ]
        , case loaded.comments of
            NotAsked ->
                text ""

            RemoteData.Loading ->
                LoadingSpinner.view

            Failure _ ->
                div [ class "rounded-lg bg-error-50 p-3 text-sm text-error-700" ]
                    [ text "コメントの取得に失敗しました。" ]

            Success comments ->
                div [ class "space-y-4" ]
                    [ viewCommentList comments
                    , viewCommentForm loaded.newCommentBody loaded.isPostingComment
                    ]
        ]


viewCommentList : List WorkflowComment -> Html Msg
viewCommentList comments =
    if List.isEmpty comments then
        p [ class "text-sm text-secondary-500" ] [ text "コメントはまだありません。" ]

    else
        div [ class "space-y-3" ]
            (List.map viewCommentItem comments)


viewCommentItem : WorkflowComment -> Html Msg
viewCommentItem commentData =
    div [ class "rounded-lg border border-secondary-200 bg-white p-3" ]
        [ div [ class "flex items-center justify-between text-xs text-secondary-500" ]
            [ span [ class "font-medium text-secondary-700" ] [ text commentData.postedBy.name ]
            , span [] [ text commentData.createdAt ]
            ]
        , p [ class "mt-1 text-sm text-secondary-900 whitespace-pre-wrap" ] [ text commentData.body ]
        ]


viewCommentForm : String -> Bool -> Html Msg
viewCommentForm body isPosting =
    div [ class "space-y-2" ]
        [ textarea
            [ class "w-full rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            , value body
            , onInput UpdateNewComment
            , placeholder "コメントを入力..."
            , rows 3
            , disabled isPosting
            ]
            []
        , div [ class "flex justify-end" ]
            [ Button.view
                { variant = Button.Primary
                , disabled = isPosting || String.isEmpty (String.trim body)
                , onClick = SubmitComment
                }
                [ text
                    (if isPosting then
                        "投稿中..."

                     else
                        "コメントを投稿"
                    )
                ]
            ]
        ]
