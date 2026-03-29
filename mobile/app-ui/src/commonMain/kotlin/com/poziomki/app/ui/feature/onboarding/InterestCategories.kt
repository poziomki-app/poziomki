package com.poziomki.app.ui.feature.onboarding

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.Barbell
import com.adamglin.phosphoricons.bold.BookOpen
import com.adamglin.phosphoricons.bold.Code
import com.adamglin.phosphoricons.bold.Compass
import com.adamglin.phosphoricons.bold.CookingPot
import com.adamglin.phosphoricons.bold.FilmSlate
import com.adamglin.phosphoricons.bold.Flask
import com.adamglin.phosphoricons.bold.GameController
import com.adamglin.phosphoricons.bold.Leaf
import com.adamglin.phosphoricons.bold.MusicNotes
import com.adamglin.phosphoricons.bold.PaintBrush
import com.adamglin.phosphoricons.bold.UsersThree

data class InterestCategoryInfo(
    val key: String,
    val displayName: String,
    val icon: ImageVector,
    val color: Color,
    val rootId: String,
)

@Suppress("LongMethod")
internal fun interestCategories(): List<InterestCategoryInfo> =
    listOf(
        InterestCategoryInfo(
            key = "sport",
            displayName = "Sport i fitness",
            icon = PhosphorIcons.Bold.Barbell,
            color = Color(0xFF6EE7B7),
            rootId = "b665ff1d-52e3-4efc-9b68-1f53d2efad10",
        ),
        InterestCategoryInfo(
            key = "muzyka",
            displayName = "Muzyka",
            icon = PhosphorIcons.Bold.MusicNotes,
            color = Color(0xFFA78BFA),
            rootId = "a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d",
        ),
        InterestCategoryInfo(
            key = "sztuka",
            displayName = "Sztuka i design",
            icon = PhosphorIcons.Bold.PaintBrush,
            color = Color(0xFFF472B6),
            rootId = "348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34",
        ),
        InterestCategoryInfo(
            key = "film",
            displayName = "Film i scena",
            icon = PhosphorIcons.Bold.FilmSlate,
            color = Color(0xFFFB7185),
            rootId = "11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8",
        ),
        InterestCategoryInfo(
            key = "technologia",
            displayName = "Technologia",
            icon = PhosphorIcons.Bold.Code,
            color = Color(0xFF22D3EE),
            rootId = "20f0febb-cfc4-4b5a-a4a4-140ff8af9abc",
        ),
        InterestCategoryInfo(
            key = "nauka",
            displayName = "Nauka i edukacja",
            icon = PhosphorIcons.Bold.Flask,
            color = Color(0xFF60A5FA),
            rootId = "7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6",
        ),
        InterestCategoryInfo(
            key = "podroze",
            displayName = "Podróże i przygody",
            icon = PhosphorIcons.Bold.Compass,
            color = Color(0xFFFB923C),
            rootId = "3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84",
        ),
        InterestCategoryInfo(
            key = "kulinaria",
            displayName = "Kulinaria",
            icon = PhosphorIcons.Bold.CookingPot,
            color = Color(0xFFFBBF24),
            rootId = "77d4f8b0-c030-4ed1-8d75-697c15a69f05",
        ),
        InterestCategoryInfo(
            key = "literatura",
            displayName = "Literatura",
            icon = PhosphorIcons.Bold.BookOpen,
            color = Color(0xFF818CF8),
            rootId = "566e1714-0ec4-4d52-8562-fca84e2c8419",
        ),
        InterestCategoryInfo(
            key = "gry",
            displayName = "Gry",
            icon = PhosphorIcons.Bold.GameController,
            color = Color(0xFF34D399),
            rootId = "a89488ea-43f1-4c72-94dd-fc3747fb95a0",
        ),
        InterestCategoryInfo(
            key = "spolecznosc",
            displayName = "Społeczność",
            icon = PhosphorIcons.Bold.UsersThree,
            color = Color(0xFFA5B4FC),
            rootId = "63318021-e21d-4d7d-a4cb-f5e0f15fc833",
        ),
        InterestCategoryInfo(
            key = "styl_zycia",
            displayName = "Styl życia",
            icon = PhosphorIcons.Bold.Leaf,
            color = Color(0xFF2DD4BF),
            rootId = "460c6106-6f65-4f0d-bbf8-ef49687ec0f3",
        ),
    )

val INTEREST_CATEGORIES: List<InterestCategoryInfo> = interestCategories()

internal val CATEGORY_MAP: Map<String, InterestCategoryInfo> =
    INTEREST_CATEGORIES.associateBy { it.key }
