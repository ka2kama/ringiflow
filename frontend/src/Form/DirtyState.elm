module Form.DirtyState exposing (clearDirty, isDirty, markDirty)

{-| Dirty フラグ管理モジュール

フォーム変更時の未保存状態を管理する。
`isDirty_` フィールドを持つ任意のモデルに対して使用できる。
extensible record 型制約により、型安全にフィールドの存在を保証する。


## 使用例

    import Form.DirtyState as DirtyState

    -- フォーム変更時
    UpdateName value ->
        let
            ( dirtyModel, dirtyCmd ) =
                DirtyState.markDirty model
        in
        ( { dirtyModel | name = value }, dirtyCmd )

    -- 保存成功時
    GotSaveResult (Ok _) ->
        let
            ( cleanModel, cleanCmd ) =
                DirtyState.clearDirty model
        in
        ( { cleanModel | submitting = False }, cleanCmd )

-}

import Ports


{-| モデルが dirty（未保存変更あり）かどうかを返す
-}
isDirty : { a | isDirty_ : Bool } -> Bool
isDirty model =
    model.isDirty_


{-| Dirty フラグを立てる（最初の変更時のみ Port を呼び出す）
-}
markDirty : { a | isDirty_ : Bool } -> ( { a | isDirty_ : Bool }, Cmd msg )
markDirty model =
    if model.isDirty_ then
        ( model, Cmd.none )

    else
        ( { model | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )


{-| Dirty フラグをクリアする（保存成功時に使用）
-}
clearDirty : { a | isDirty_ : Bool } -> ( { a | isDirty_ : Bool }, Cmd msg )
clearDirty model =
    if model.isDirty_ then
        ( { model | isDirty_ = False }
        , Ports.setBeforeUnloadEnabled False
        )

    else
        ( model, Cmd.none )
