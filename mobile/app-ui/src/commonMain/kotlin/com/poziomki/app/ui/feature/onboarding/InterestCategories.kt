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
)

val INTEREST_CATEGORIES: List<InterestCategoryInfo> =
    listOf(
        InterestCategoryInfo("sport", "Sport i fitness", PhosphorIcons.Bold.Barbell, Color(0xFF6EE7B7)),
        InterestCategoryInfo("muzyka", "Muzyka", PhosphorIcons.Bold.MusicNotes, Color(0xFFA78BFA)),
        InterestCategoryInfo("sztuka", "Sztuka i design", PhosphorIcons.Bold.PaintBrush, Color(0xFFF472B6)),
        InterestCategoryInfo("film", "Film i scena", PhosphorIcons.Bold.FilmSlate, Color(0xFFFB7185)),
        InterestCategoryInfo("technologia", "Technologia", PhosphorIcons.Bold.Code, Color(0xFF22D3EE)),
        InterestCategoryInfo("nauka", "Nauka i edukacja", PhosphorIcons.Bold.Flask, Color(0xFF60A5FA)),
        InterestCategoryInfo("podroze", "Podróże i przygody", PhosphorIcons.Bold.Compass, Color(0xFFFB923C)),
        InterestCategoryInfo("kulinaria", "Kulinaria", PhosphorIcons.Bold.CookingPot, Color(0xFFFBBF24)),
        InterestCategoryInfo("literatura", "Literatura", PhosphorIcons.Bold.BookOpen, Color(0xFF818CF8)),
        InterestCategoryInfo("gry", "Gry", PhosphorIcons.Bold.GameController, Color(0xFF34D399)),
        InterestCategoryInfo("spolecznosc", "Społeczność", PhosphorIcons.Bold.UsersThree, Color(0xFFA5B4FC)),
        InterestCategoryInfo("styl_zycia", "Styl życia", PhosphorIcons.Bold.Leaf, Color(0xFF2DD4BF)),
    )

internal val CATEGORY_MAP: Map<String, InterestCategoryInfo> =
    INTEREST_CATEGORIES.associateBy { it.key }
