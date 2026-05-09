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
        "openmaptiles": { "type": "vector", "url": "https://tiles.openfreemap.org/planet" },
        "uw_campuses": { "type": "geojson", "data": $WARSAW_CAMPUSES_GEOJSON },
        "uw_campus_labels": { "type": "geojson", "data": $WARSAW_CAMPUS_LABELS_GEOJSON },
        "metro": { "type": "geojson", "data": $WARSAW_METRO_GEOJSON }
      },
      "layers": [
        { "id": "background", "type": "background",
          "paint": { "background-color": "#FFFFFF" } },

        { "id": "landcover_wood", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "wood"],
          "paint": { "fill-color": "#B7D49A" } },
        { "id": "landcover_grass", "type": "fill", "source": "openmaptiles", "source-layer": "landcover",
          "filter": ["==", "class", "grass"],
          "paint": { "fill-color": "#CDE3B0" } },

        { "id": "park", "type": "fill", "source": "openmaptiles", "source-layer": "park",
          "paint": { "fill-color": "#A9CF94" } },
        { "id": "park_outline", "type": "line", "source": "openmaptiles", "source-layer": "park",
          "paint": { "line-color": "#7DB36A", "line-width": 0.8, "line-opacity": 0.6 } },
        { "id": "park_label", "type": "symbol", "source": "openmaptiles", "source-layer": "park",
          "minzoom": 12,
          "layout": {
            "text-field": ["coalesce", ["get", "name:pl"], ["get", "name:latin"], ["get", "name"]],
            "text-font": ["Nunito Regular"],
            "text-size": ["interpolate", ["linear"], ["zoom"], 12, 10, 16, 13],
            "text-letter-spacing": 0.02
          },
          "paint": { "text-color": "#3C5C2A" } },

        { "id": "landuse_school", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["in", "class", "school", "university"],
          "paint": { "fill-color": "#E0EBF5", "fill-opacity": 0.8 } },
        { "id": "landuse_residential", "type": "fill", "source": "openmaptiles", "source-layer": "landuse",
          "filter": ["==", "class", "residential"],
          "paint": { "fill-color": "#F4F4F4", "fill-opacity": 0.7 } },

        { "id": "water", "type": "fill", "source": "openmaptiles", "source-layer": "water",
          "paint": { "fill-color": "#BBE0E6" } },

        { "id": "roads_minor", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "minzoom": 15,
          "filter": ["in", "class", "minor", "service"],
          "paint": { "line-color": "#E8E8E8", "line-width": 1, "line-opacity": 0.6 } },

        { "id": "roads_major_casing", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk"],
          "paint": { "line-color": "#D8DFE5", "line-width": 3.5, "line-opacity": 0.7 } },
        { "id": "roads_major", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["in", "class", "primary", "secondary", "tertiary", "trunk"],
          "paint": { "line-color": "#FFFFFF", "line-width": 2.2, "line-opacity": 0.9 } },

        { "id": "roads_motorway", "type": "line", "source": "openmaptiles", "source-layer": "transportation",
          "filter": ["==", "class", "motorway"],
          "paint": { "line-color": "#FFE0B7", "line-width": 3.5, "line-opacity": 0.8 } },

        { "id": "campus_buildings_3d", "type": "fill-extrusion", "source": "openmaptiles", "source-layer": "building",
          "minzoom": 15,
          "filter": ["within", $WARSAW_CAMPUSES_GEOJSON],
          "paint": {
            "fill-extrusion-color": "#7AB6D2",
            "fill-extrusion-height": ["coalesce", ["get", "render_height"], 8],
            "fill-extrusion-base": ["coalesce", ["get", "render_min_height"], 0],
            "fill-extrusion-opacity": 0.85
          } },

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
          "paint": { "text-color": "#1F2A33" } },

        { "id": "uw_campuses_fill", "type": "fill", "source": "uw_campuses",
          "filter": ["==", ["geometry-type"], "Polygon"],
          "paint": { "fill-color": "#B8DCEA", "fill-opacity": 0.55 } },

        { "id": "uw_campus_labels", "type": "symbol", "source": "uw_campus_labels",
          "layout": {
            "text-field": ["get", "name"],
            "text-font": ["Montserrat ExtraBold"],
            "text-size": ["interpolate", ["linear"], ["zoom"], 11, 11, 16, 16],
            "text-letter-spacing": 0.02,
            "text-allow-overlap": false
          },
          "paint": { "text-color": "#1B4D63" } },

        { "id": "buw_dot", "type": "circle", "source": "uw_campuses",
          "filter": ["all", ["==", ["geometry-type"], "Point"], ["==", ["get", "name"], "BUW"]],
          "paint": {
            "circle-radius": 5,
            "circle-color": "#1B4D63",
            "circle-stroke-color": "#FFFFFF",
            "circle-stroke-width": 2
          } },
        { "id": "buw_label", "type": "symbol", "source": "uw_campuses",
          "filter": ["all", ["==", ["geometry-type"], "Point"], ["==", ["get", "name"], "BUW"]],
          "layout": {
            "text-field": "BUW",
            "text-font": ["Montserrat ExtraBold"],
            "text-size": 12,
            "text-anchor": "top",
            "text-offset": [0, 0.7]
          },
          "paint": { "text-color": "#1B4D63" } },

        { "id": "metro_circle", "type": "circle", "source": "metro",
          "paint": {
            "circle-radius": 7,
            "circle-color": "#F5C518",
            "circle-stroke-color": "#FFFFFF",
            "circle-stroke-width": 1.2
          } },
        { "id": "metro_letter", "type": "symbol", "source": "metro",
          "layout": {
            "text-field": "M",
            "text-font": ["Montserrat ExtraBold"],
            "text-size": 10,
            "text-allow-overlap": true
          },
          "paint": { "text-color": "#D6311A" } },
        { "id": "metro_name", "type": "symbol", "source": "metro",
          "minzoom": 13,
          "layout": {
            "text-field": ["get", "name"],
            "text-font": ["Nunito Regular"],
            "text-size": 11,
            "text-anchor": "top",
            "text-offset": [0, 1.0],
            "text-allow-overlap": false
          },
          "paint": { "text-color": "#5A5A5A" } }
      ]
    }
    """.trimIndent()
