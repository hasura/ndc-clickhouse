SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("field1" String)))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(groupArray(tuple("_row"."_field_field1"))) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."ColumnA" AS "_field_field1"
        FROM
          "Schema1"."Table1" AS "_origin"
        WHERE
          (
            "_origin"."ColumnB" = [('foo', 'bar')]
            AND "_origin"."ColumnC" = [('foo', 'bar')]
            AND "_origin"."ColumnD" = ((1, 'foo'))
            AND "_origin"."ColumnE" = ([(1, 'foo')])
            AND "_origin"."ColumnF" = ((1, 'foo', [(2, 'bar')]))
            AND "_origin"."ColumnG" = (
              NULL,
              { 'foo': 'bar' },
              [('foo', ('foo', 'bar'))],
              ('foo', 'bar')
            )
          )
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;