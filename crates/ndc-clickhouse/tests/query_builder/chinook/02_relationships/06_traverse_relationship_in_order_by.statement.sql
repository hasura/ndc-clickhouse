SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("trackId" Int32, "name" String, "Album" Tuple(rows Array(Tuple("Artist" Tuple(rows Array(Tuple("name" Nullable(String))))))))))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(
        groupArray(
          tuple(
            "_row"."_field_trackId",
            "_row"."_field_name",
            "_row"."_field_Album"
          )
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."TrackId" AS "_field_trackId",
          "_origin"."Name" AS "_field_name",
          "_rel_0_Album"."_rowset" AS "_field_Album"
        FROM
          "Chinook"."Track" AS "_origin"
          LEFT JOIN (
            SELECT
              tuple(groupArray(tuple("_row"."_field_Artist"))) AS "_rowset",
              "_row"."_relkey_AlbumId" AS "_relkey_AlbumId"
            FROM
              (
                SELECT
                  "_rel_0_Artist"."_rowset" AS "_field_Artist",
                  "_origin"."AlbumId" AS "_relkey_AlbumId"
                FROM
                  "Chinook"."Album" AS "_origin"
                  LEFT JOIN (
                    SELECT
                      tuple(groupArray(tuple("_row"."_field_name"))) AS "_rowset",
                      "_row"."_relkey_ArtistId" AS "_relkey_ArtistId"
                    FROM
                      (
                        SELECT
                          "_origin"."Name" AS "_field_name",
                          "_origin"."ArtistId" AS "_relkey_ArtistId"
                        FROM
                          "Chinook"."Artist" AS "_origin"
                      ) AS "_row"
                    GROUP BY
                      "_row"."_relkey_ArtistId"
                  ) AS "_rel_0_Artist" ON "_origin"."ArtistId" = "_rel_0_Artist"."_relkey_ArtistId"
              ) AS "_row"
            GROUP BY
              "_row"."_relkey_AlbumId"
          ) AS "_rel_0_Album" ON "_origin"."AlbumId" = "_rel_0_Album"."_relkey_AlbumId"
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
            LIMIT
              1 BY "_order_by_0"."AlbumId"
          ) AS "_order_by_0" ON "_origin"."AlbumId" = "_order_by_0"."_relkey_AlbumId"
        ORDER BY
          "_order_by_0"."_order_by_value" ASC,
          "_origin"."Name" ASC
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;