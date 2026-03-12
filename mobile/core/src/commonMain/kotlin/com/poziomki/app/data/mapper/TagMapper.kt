package com.poziomki.app.data.mapper

import com.poziomki.app.network.Tag

fun Tag.toDbParams(): List<Any?> =
    listOf(
        id,
        name,
        scope,
        category,
        emoji,
        parentId,
    )

fun com.poziomki.app.db.Tag.toApiModel(): Tag =
    Tag(
        id = id,
        name = name,
        scope = scope,
        category = category,
        emoji = emoji,
        parentId = parent_id,
    )
