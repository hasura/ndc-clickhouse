SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("albumId" Int32, "artistId" Int32, "title" String, "Tracks" Tuple(rows Array(Tuple("trackId" Int32, "name" String, "unitPrice" Float64))))))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(
        groupArray(
          tuple(
            "_row"."_field_albumId",
            "_row"."_field_artistId",
            "_row"."_field_title",
            "_row"."_field_Tracks"
          )
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."AlbumId" AS "_field_albumId",
          "_origin"."ArtistId" AS "_field_artistId",
          "_origin"."Title" AS "_field_title",
          "_rel_0_Tracks"."_rowset" AS "_field_Tracks"
        FROM
          "Chinook"."Album" AS "_origin"
          LEFT JOIN (
            SELECT
              tuple(
                groupArray(
                  tuple(
                    "_row"."_field_trackId",
                    "_row"."_field_name",
                    "_row"."_field_unitPrice"
                  )
                )
              ) AS "_rowset",
              "_row"."_relkey_AlbumId" AS "_relkey_AlbumId"
            FROM
              (
                SELECT
                  "_origin"."TrackId" AS "_field_trackId",
                  "_origin"."Name" AS "_field_name",
                  "_origin"."UnitPrice" AS "_field_unitPrice",
                  "_origin"."AlbumId" AS "_relkey_AlbumId"
                FROM
                  "Chinook"."Track" AS "_origin"
              ) AS "_row"
            GROUP BY
              "_row"."_relkey_AlbumId"
          ) AS "_rel_0_Tracks" ON "_origin"."AlbumId" = "_rel_0_Tracks"."_relkey_AlbumId"
          LEFT JOIN (
            SELECT
              TRUE AS "_exists_0",
              "_exists_1"."ArtistId" AS "_relkey_ArtistId"
            FROM
              "Chinook"."Artist" AS "_exists_1"
            WHERE
              "_exists_1"."Name" = { p0 :Nullable(String) }
            LIMIT
              1 BY "_exists_1"."ArtistId"
          ) AS "_exists_0" ON "_origin"."ArtistId" = "_exists_0"."_relkey_ArtistId"
        WHERE
          "_exists_0"."_exists_0" = TRUE
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;