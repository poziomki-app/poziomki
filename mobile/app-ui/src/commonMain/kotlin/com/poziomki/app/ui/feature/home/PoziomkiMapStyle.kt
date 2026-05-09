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
      "glyphs": "https://poziomki.app/fonts/{fontstack}/{range}.pbf",
      "sprite": "https://tiles.openfreemap.org/sprites/ofm_f384/ofm",
      "sources": {
        "openmaptiles": { "type": "vector", "url": "https://tiles.openfreemap.org/planet" }
      },
      "layers": [
        { "id": "background", "type": "background",
          "paint": { "background-color": "#FFFFFF" } },

        { "id": "landcover_wood", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "wood"],
          "paint": { "fill-color": "#D8E6CC", "fill-opacity": 0.7 } },
        { "id": "landcover_grass", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "grass"],
          "paint": { "fill-color": "#E5EFD8", "fill-opacity": 0.7 } },

        { "id": "park", "type": "fill", "source": "openmaptiles", "source-layer": "park",
          "paint": { "fill-color": "#DDEAD0" } },

        { "id": "landuse_school", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["in", "class", "school", "university"],
          "paint": { "fill-color": "#E0EBF5", "fill-opacity": 0.8 } },
        { "id": "landuse_residential", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["==", "class", "residential"],
          "paint": { "fill-color": "#F4F4F4", "fill-opacity": 0.7 } },

        { "id": "water", "type": "fill", "source": "openmaptiles", "source-layer": "water",
          "paint": { "fill-color": "#BBE0E6" } },

        { "id": "buildings", "type": "fill", "source": "openmaptiles", "source-layer": "building",
          "minzoom": 13,
          "paint": { "fill-color": "#D6D6D6", "fill-outline-color": "#C2C2C2" } },

        { "id": "roads_minor_casing", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "minzoom": 14,
          "filter": ["in", "class", "minor", "service", "track"],
          "paint": { "line-color": "#E2E2E2", "line-width": 2 } },
        { "id": "roads_minor", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "minzoom": 14,
          "filter": ["in", "class", "minor", "service", "track"],
          "paint": { "line-color": "#FFFFFF", "line-width": 1.4 } },

        { "id": "roads_major_casing", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk"],
          "paint": { "line-color": "#C9D6E0", "line-width": 4.5 } },
        { "id": "roads_major", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk"],
          "paint": { "line-color": "#FFFFFF", "line-width": 3 } },

        { "id": "roads_motorway_casing", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["==", "class", "motorway"],
          "paint": { "line-color": "#E8B98A", "line-width": 6 } },
        { "id": "roads_motorway", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["==", "class", "motorway"],
          "paint": { "line-color": "#FFE0B7", "line-width": 4 } },

        { "id": "street_labels_major", "type": "symbol", "source": "openmaptiles", "source-layer": "transportation_name",
          "minzoom": 13,
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk", "motorway"],
          "layout": {
            "text-field": ["coalesce", ["get", "name:pl"], ["get", "name:latin"], ["get", "name"]],
            "text-font": ["Nunito Regular"],
            "symbol-placement": "line",
            "text-size": ["interpolate", ["linear"], ["zoom"], 13, 10, 18, 14],
            "text-letter-spacing": 0.04
          },
          "paint": { "text-color": "#5A5A5A", "text-halo-color": "#FFFFFF", "text-halo-width": 1.6 } },

        { "id": "place_labels", "type": "symbol", "source": "openmaptiles", "source-layer": "place",
          "filter": ["in", "class", "neighbourhood", "suburb"],
          "layout": {
            "text-field": ["coalesce", ["get", "name:pl"], ["get", "name:latin"], ["get", "name"]],
            "text-font": ["Montserrat ExtraBold"],
            "text-size": ["interpolate", ["linear"], ["zoom"], 11, 12, 16, 20],
            "text-letter-spacing": 0.08,
            "text-transform": "uppercase"
          },
          "paint": { "text-color": "#1F2A33", "text-halo-color": "#FFFFFF", "text-halo-width": 2.0 } }
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
