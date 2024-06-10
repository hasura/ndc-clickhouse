SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("field1" String, "field2" Tuple("b" Map(String, String), "c" Array(Tuple("a" String, "b" Tuple(String, String))), "d" Tuple("a" String, "b" String)))))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(
        groupArray(
          tuple("_row"."_field_field1", "_row"."_field_field2")
        )
      ) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."ColumnA" AS "_field_field1",
          tuple(
            "_origin"."ColumnG"."b",
            "_origin"."ColumnG"."c",
            "_origin"."ColumnG"."d"
          ) AS "_field_field2"
        FROM
          "Schema1"."Table1" AS "_origin"
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;