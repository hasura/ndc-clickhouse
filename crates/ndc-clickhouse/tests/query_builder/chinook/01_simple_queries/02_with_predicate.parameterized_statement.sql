SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("albumId" Int32, "artistId" Int32, "title" String)))'
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
            "_row"."_field_title"
          )
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."AlbumId" AS "_field_albumId",
          "_origin"."ArtistId" AS "_field_artistId",
          "_origin"."Title" AS "_field_title"
        FROM
          "Chinook"."Album" AS "_origin"
        WHERE
          "_origin"."ArtistId" = { p0 :Int32 }
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;