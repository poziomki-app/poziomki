@file:Suppress("MaxLineLength", "MaximumLineLength")

package com.poziomki.app.ui.icons

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

private fun mcBuilder(name: String) =
    ImageVector.Builder(
        name = name,
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 24f,
        viewportHeight = 24f,
    )

private fun ImageVector.Builder.evenOdd(path: String): ImageVector.Builder {
    addPath(
        pathData = addPathNodes(path),
        pathFillType = PathFillType.EvenOdd,
        fill = SolidColor(Color.Black),
    )
    return this
}

private object McPaths {
    const val USERS_FILL =
        "M9.5 12a4.5 4.5 0 1 0 0-9a4.5 4.5 0 0 0 0 9M21 9a3 3 0 1 1-6 0a3 3 0 0 1 6 0M9.5 13c1.993 0 3.805.608 5.137 1.466c.667.43 1.238.937 1.653 1.49c.407.545.71 1.2.71 1.901c0 .755-.35 1.36-.864 1.797c-.485.41-1.117.676-1.77.859c-1.313.367-3.05.487-4.866.487s-3.553-.12-4.865-.487c-.654-.183-1.286-.449-1.77-.859C2.349 19.218 2 18.612 2 17.857c0-.702.303-1.356.71-1.9c.415-.554.986-1.062 1.653-1.49C5.695 13.607 7.507 13 9.5 13m8.5 0c1.32 0 2.518.436 3.4 1.051c.822.573 1.6 1.477 1.6 2.52c0 .587-.253 1.073-.638 1.426c-.357.328-.809.528-1.244.66c-.87.263-1.99.343-3.118.343h-.203c.13-.348.203-.73.203-1.143c0-.99-.423-1.85-.91-2.5c-.486-.649-1.13-1.22-1.849-1.691A6.06 6.06 0 0 1 18 13"

    const val CAL_FILL =
        "M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-1M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
}

object MingcuteNavIcons {
    val UsersFill: ImageVector by lazy { mcBuilder("McUsersFill").evenOdd(McPaths.USERS_FILL).build() }
    val CalendarFill: ImageVector by lazy { mcBuilder("McCalFill").evenOdd(McPaths.CAL_FILL).build() }
}
