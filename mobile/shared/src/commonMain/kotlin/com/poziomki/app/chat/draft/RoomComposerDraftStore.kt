package com.poziomki.app.chat.draft

interface RoomComposerDraftStore {
    fun getDraft(roomId: String): String

    fun saveDraft(
        roomId: String,
        draft: String,
    )

    fun clearDraft(roomId: String)
}
