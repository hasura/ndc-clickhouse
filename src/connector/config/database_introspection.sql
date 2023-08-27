SELECT
    t.table_name AS "table_name",
    t.table_schema AS "table_schema",
    t.table_catalog AS "table_catalog",
    st.primary_key AS "primary_key",
    toString(
        cast(
            t.table_type,
            'Enum(\'table\' = 1, \'view\' = 2)'
        )
    ) AS "table_type",
    cast(
      c.columns,
      'Array(Tuple(column_name String, data_type String, is_nullable Bool, is_in_primary_key Bool))'
    ) AS "columns"
FROM INFORMATION_SCHEMA.TABLES AS t
    LEFT JOIN system.tables AS st ON st.database = t.table_schema AND st.name = t.table_name 
    LEFT JOIN (
        SELECT c.table_catalog,
            c.table_schema,
            c.table_name,
            groupArray(
                tuple(
                    c.column_name,
                    c.data_type,
                    toBool(c.is_nullable),
                    toBool(sc.is_in_primary_key)
                )
            ) AS "columns"
        FROM INFORMATION_SCHEMA.COLUMNS AS c
        LEFT JOIN system.columns AS sc ON sc.database = c.table_schema AND sc.table = c.table_name AND sc.name = c.column_name
        GROUP BY c .table_catalog,
            c.table_schema,
            c.table_name
    ) AS c USING (table_catalog, table_schema, table_name)
WHERE t.table_catalog = currentDatabase()
    AND t.table_type IN (1, 2) -- table type is an enum, where tables and views are 1 and 2 respectively
FORMAT JSON;