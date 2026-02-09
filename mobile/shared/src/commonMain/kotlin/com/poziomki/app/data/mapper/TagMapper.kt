package com.poziomki.app.data.mapper

import com.poziomki.app.api.Tag

fun Tag.toDbParams(): List<Any?> =
    listOf(
        id,
        name,
        scope,
        category,
        emoji,
    )

fun com.poziomki.app.db.Tag.toApiModel(): Tag =
    Tag(
        id = id,
        name = name,
        scope = scope,
        category = category,
        emoji = emoji,
    )
