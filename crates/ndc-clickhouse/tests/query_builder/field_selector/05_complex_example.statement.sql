SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("field1" String, "field2" Tuple("child" Tuple("id" UInt32, "name" String, "child" Tuple(rows Array(Tuple("name" String))), "toys" Array(Tuple("name" String)))))))'
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
            tuple(
              "_origin"."ColumnF"."child"."id",
              "_origin"."ColumnF"."child"."name",
              "_rel_0_child"."_rowset",
              arrayMap(
                (_value) -> tuple(_value."name"),
                "_origin"."ColumnF"."child"."toys"
              )
            )
          ) AS "_field_field2"
        FROM
          "Schema1"."Table1" AS "_origin"
          LEFT JOIN (
            SELECT
              tuple(groupArray(tuple("_row"."_field_name"))) AS "_rowset",
              "_row"."_relkey_Id" AS "_relkey_Id"
            FROM
              (
                SELECT
                  "_origin"."Name" AS "_field_name",
                  "_origin"."Id" AS "_relkey_Id"
                FROM
                  "Schema1"."Table2" AS "_origin"
              ) AS "_row"
            GROUP BY
              "_row"."_relkey_Id"
          ) AS "_rel_0_child" ON "_origin"."ColumnF"."child"."id" = "_rel_0_child"."_relkey_Id"
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;