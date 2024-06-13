SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("trackId" Int32, "name" String)))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(
        groupArray(
          tuple("_row"."_field_trackId", "_row"."_field_name")
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."TrackId" AS "_field_trackId",
          "_origin"."Name" AS "_field_name"
        FROM
          "Chinook"."Track" AS "_origin"
          LEFT JOIN (
            SELECT
              "_order_by_0"."AlbumId" AS "_relkey_AlbumId",
              "_order_by_1"."Name" AS "_order_by_value"
            FROM
              "Chinook"."Album" AS "_order_by_0"
              JOIN "Chinook"."Artist" AS "_order_by_1" ON "_order_by_0"."ArtistId" = "_order_by_1"."ArtistId"
            WHERE
              TRUE
              AND TRUE
            GROUP BY
              "_order_by_0"."AlbumId",
              "_order_by_1"."Name"
            LIMIT
              1 BY "_order_by_0"."AlbumId"
          ) AS "_order_by_0" ON "_origin"."AlbumId" = "_order_by_0"."_relkey_AlbumId"
          LEFT JOIN (
            SELECT
              "_order_by_0"."AlbumId" AS "_relkey_AlbumId",
              COUNT(*) AS "_order_by_value"
            FROM
              "Chinook"."Album" AS "_order_by_0"
              JOIN "Chinook"."Artist" AS "_order_by_1" ON "_order_by_0"."ArtistId" = "_order_by_1"."ArtistId"
            WHERE
              TRUE
              AND TRUE
            GROUP BY
              "_order_by_0"."AlbumId"
            LIMIT
              1 BY "_order_by_0"."AlbumId"
          ) AS "_order_by_1" ON "_origin"."AlbumId" = "_order_by_1"."_relkey_AlbumId"
          LEFT JOIN (
            SELECT
              "_order_by_0"."AlbumId" AS "_relkey_AlbumId",
              max("_order_by_1"."Name") AS "_order_by_value"
            FROM
              "Chinook"."Album" AS "_order_by_0"
              JOIN "Chinook"."Artist" AS "_order_by_1" ON "_order_by_0"."ArtistId" = "_order_by_1"."ArtistId"
            WHERE
              TRUE
              AND TRUE
            GROUP BY
              "_order_by_0"."AlbumId"
            LIMIT
              1 BY "_order_by_0"."AlbumId"
          ) AS "_order_by_2" ON "_origin"."AlbumId" = "_order_by_2"."_relkey_AlbumId"
        ORDER BY
          "_order_by_0"."_order_by_value" ASC,
          "_order_by_1"."_order_by_value" ASC,
          "_order_by_2"."_order_by_value" ASC
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;