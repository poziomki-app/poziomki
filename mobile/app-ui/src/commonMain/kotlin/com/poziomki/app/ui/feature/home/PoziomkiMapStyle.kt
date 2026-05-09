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
          "paint": { "text-color": "#5A5A5A" } },

        { "id": "place_labels", "type": "symbol", "source": "openmaptiles", "source-layer": "place",
          "filter": ["in", "class", "neighbourhood", "suburb"],
          "layout": {
            "text-field": ["coalesce", ["get", "name:pl"], ["get", "name:latin"], ["get", "name"]],
            "text-font": ["Montserrat ExtraBold"],
            "text-size": ["interpolate", ["linear"], ["zoom"], 11, 12, 16, 20],
            "text-letter-spacing": 0.04,
            "text-transform": "lowercase"
          },
          "paint": { "text-color": "#1F2A33" } }
      ]
    }
    """.trimIndent()
