package com.poziomki.app.chat.draft

class InMemoryRoomComposerDraftStore : RoomComposerDraftStore {
    private val draftsByRoomId = mutableMapOf<String, String>()

    override fun getDraft(roomId: String): String = draftsByRoomId[roomId].orEmpty()

    override fun saveDraft(
        roomId: String,
        draft: String,
    ) {
        if (draft.isBlank()) {
            draftsByRoomId.remove(roomId)
            return
        }
        draftsByRoomId[roomId] = draft
    }

    override fun clearDraft(roomId: String) {
        draftsByRoomId.remove(roomId)
    }
}
