SELECT
    t.table_name AS "table_name",
    t.table_schema AS "table_schema",
    t.table_catalog AS "table_catalog",
    if(empty(st.primary_key), null, st.primary_key)  AS "primary_key",
    toString(t.table_type) as "table_type",
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
    ) AS c ON t.table_catalog = c.table_catalog AND t.table_schema = c.table_schema AND t.table_name = c.table_name
WHERE t.table_catalog NOT IN ('system', 'INFORMATION_SCHEMA', 'information_schema')
FORMAT JSON;