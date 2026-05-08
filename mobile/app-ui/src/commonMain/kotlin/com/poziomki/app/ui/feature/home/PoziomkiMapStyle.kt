package com.poziomki.app.ui.feature.home

/**
 * Custom MapLibre style — pastel palette, OpenFreeMap vector tiles,
 * 2.5D buildings at zoom >= 15. Inlined as JSON so the app stays
 * self-contained (no extra hosting). Layer order matters: water →
 * landcover → landuse → buildings → roads → labels.
 *
 * Field names (`class`, `subclass`, `render_height`) match OpenMapTiles
 * schema, which OpenFreeMap serves at /planet.
 */
@Suppress("MaxLineLength", "LongMethod")
internal val POZIOMKI_MAP_STYLE_JSON: String =
    """
    {
      "version": 8,
      "name": "Poziomki Pastel",
      "glyphs": "https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf",
      "sprite": "https://tiles.openfreemap.org/sprites/ofm_f384/ofm",
      "sources": {
        "openmaptiles": { "type": "vector", "url": "https://tiles.openfreemap.org/planet" }
      },
      "layers": [
        { "id": "background", "type": "background",
          "paint": { "background-color": "#FAF6F0" } },

        { "id": "landcover_wood", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "wood"],
          "paint": { "fill-color": "#D2E1C2", "fill-opacity": 0.6 } },
        { "id": "landcover_grass", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "grass"],
          "paint": { "fill-color": "#DFEBC7", "fill-opacity": 0.6 } },

        { "id": "park", "type": "fill", "source": "openmaptiles", "source-layer": "park",
          "paint": { "fill-color": "#D9E8C9" } },

        { "id": "landuse_school", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["in", "class", "school", "university"],
          "paint": { "fill-color": "#F4DCC9", "fill-opacity": 0.7 } },
        { "id": "landuse_residential", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["==", "class", "residential"],
          "paint": { "fill-color": "#F1ECE2", "fill-opacity": 0.5 } },

        { "id": "water", "type": "fill", "source": "openmaptiles", "source-layer": "water",
          "paint": { "fill-color": "#BFD8EF" } },

        { "id": "buildings", "type": "fill", "source": "openmaptiles", "source-layer": "building",
          "minzoom": 13,
          "paint": { "fill-color": "#EAE2D2", "fill-outline-color": "#D9D0BD" } },

        { "id": "roads_minor", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "minzoom": 13,
          "filter": ["in", "class", "minor", "service", "track"],
          "paint": { "line-color": "#FFFFFF", "line-width": 1.5 } },
        { "id": "roads_major", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk"],
          "paint": { "line-color": "#FFFFFF", "line-width": 3 } },
        { "id": "roads_motorway", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["==", "class", "motorway"],
          "paint": { "line-color": "#FFE6BF", "line-width": 4 } },

        { "id": "place_labels", "type": "symbol", "source": "openmaptiles", "source-layer": "place",
          "filter": ["in", "class", "city", "town", "neighbourhood"],
          "layout": {
            "text-field": ["coalesce", ["get", "name:pl"], ["get", "name:latin"], ["get", "name"]],
            "text-font": ["Noto Sans Regular"],
            "text-size": ["interpolate", ["linear"], ["zoom"], 8, 11, 16, 18]
          },
          "paint": { "text-color": "#3D3530", "text-halo-color": "#FAF6F0", "text-halo-width": 1.6 } }
      ]
    }
    """.trimIndent()

/**
 * Major Polish university campuses with rough centroids. Used to surface
 * the "kampus tutaj" markers on the nearby map. Coordinates are the main
 * building / central courtyard — close enough to be recognisable from the
 * surrounding street layout without polygon overlays.
 */
internal data class Campus(
    val name: String,
    val lat: Double,
    val lng: Double,
)

internal val POLISH_CAMPUSES: List<Campus> =
    listOf(
        // Warsaw
        Campus("UW", 52.2407, 21.0156),
        Campus("PW", 52.2207, 21.0098),
        Campus("SGH", 52.2099, 21.0114),
        Campus("WUM", 52.2178, 21.0249),
        Campus("AWF", 52.2638, 20.9569),
        Campus("UKSW", 52.2839, 20.9537),
        // Kraków
        Campus("UJ", 50.0614, 19.9366),
        Campus("AGH", 50.0647, 19.9229),
        // Poznań
        Campus("UAM", 52.4646, 16.9249),
        Campus("PP", 52.4053, 16.9501),
        // Wrocław
        Campus("UWr", 51.1140, 17.0314),
        Campus("PWr", 51.1078, 17.0590),
        // Łódź
        Campus("UŁ", 51.7593, 19.4574),
        Campus("PŁ", 51.7470, 19.4530),
        // Gdańsk
        Campus("UG", 54.3812, 18.6037),
        Campus("PG", 54.3717, 18.6190),
    )
