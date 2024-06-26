SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("albumId" Int32, "artistId" Int32, "title" String, "Artist" Tuple(rows Array(Tuple("artistId" Int32, "name" Nullable(String)))))))'
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
            "_row"."_field_Artist"
          )
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."AlbumId" AS "_field_albumId",
          "_origin"."ArtistId" AS "_field_artistId",
          "_origin"."Title" AS "_field_title",
          "_rel_0_Artist"."_rowset" AS "_field_Artist"
        FROM
          "Chinook"."Album" AS "_origin"
          LEFT JOIN (
            SELECT
              tuple(
                groupArray(
                  tuple("_row"."_field_artistId", "_row"."_field_name")
                )
              ) AS "_rowset",
              "_row"."_relkey_ArtistId" AS "_relkey_ArtistId"
            FROM
              (
                SELECT
                  "_origin"."ArtistId" AS "_field_artistId",
                  "_origin"."Name" AS "_field_name",
                  "_origin"."ArtistId" AS "_relkey_ArtistId"
                FROM
                  "Chinook"."Artist" AS "_origin"
              ) AS "_row"
            GROUP BY
              "_row"."_relkey_ArtistId"
          ) AS "_rel_0_Artist" ON "_origin"."ArtistId" = "_rel_0_Artist"."_relkey_ArtistId"
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;