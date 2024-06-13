WITH "_vars" AS (
  SELECT
    *
  FROM
    format(JSONColumns, '{"_varset_id":[]}')
)
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
  "_vars" AS "_vars"
  LEFT JOIN (
    SELECT
      tuple(
        groupArray(
          tuple(
            "_row"."_field_albumId",
            "_row"."_field_artistId",
            "_row"."_field_title"
          )
        )
      ) AS "_rowset",
      "_row"."_varset_id" AS "_varset_id"
    FROM
      (
        SELECT
          "_origin"."AlbumId" AS "_field_albumId",
          "_origin"."ArtistId" AS "_field_artistId",
          "_origin"."Title" AS "_field_title",
          "_vars"."_varset_id" AS "_varset_id"
        FROM
          "_vars" AS "_vars"
          CROSS JOIN "Chinook"."Album" AS "_origin"
        WHERE
          "_origin"."ArtistId" = "_vars"."_var_ArtistId"
      ) AS "_row"
    GROUP BY
      "_row"."_varset_id"
  ) AS "_rowset" ON "_vars"."_varset_id" = "_rowset"."_varset_id"
ORDER BY
  "_vars"."_varset_id" ASC FORMAT TabSeparatedRaw;