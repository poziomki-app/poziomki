package com.poziomki.app.chat.draft

import com.poziomki.app.db.PoziomkiDatabase
import kotlinx.datetime.Clock

class SqlDelightRoomComposerDraftStore(
    private val db: PoziomkiDatabase,
) : RoomComposerDraftStore {
    override fun getDraft(roomId: String): String =
        db.chatDraftQueries
            .selectByRoomId(roomId)
            .executeAsOneOrNull()
            .orEmpty()

    override fun saveDraft(
        roomId: String,
        draft: String,
    ) {
        if (draft.isBlank()) {
            db.chatDraftQueries.deleteByRoomId(roomId)
            return
        }
        db.chatDraftQueries.upsert(
            room_id = roomId,
            draft = draft,
            updated_at = Clock.System.now().toEpochMilliseconds(),
        )
    }

    override fun clearDraft(roomId: String) {
        db.chatDraftQueries.deleteByRoomId(roomId)
    }
}
