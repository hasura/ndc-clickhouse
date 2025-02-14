SELECT toJSONString(
    cast(
        groupArray(
            tuple(
                t.table_name,
                t.table_schema,
                t.table_catalog,
                t.table_comment,
                if(empty(st.primary_key), null, st.primary_key),
                toString(t.table_type),
                v.view_definition,
                c.columns
            )
        ),
        'Array(Tuple(table_name String, table_schema String, table_catalog String, table_comment Nullable(String), primary_key Nullable(String), table_type String, view_definition String, columns Array(Tuple(column_name String, data_type String, is_nullable Bool, is_in_primary_key Bool, column_comment String))))'
    )
)
FROM INFORMATION_SCHEMA.TABLES AS t
    LEFT JOIN INFORMATION_SCHEMA.VIEWS AS v ON v.table_schema = t.table_schema
            AND v.table_name = t.table_name
    LEFT JOIN system.tables AS st ON st.database = t.table_schema
    AND st.name = t.table_name
    LEFT JOIN (
        SELECT c.table_catalog,
            c.table_schema,
            c.table_name,
            groupArray(
                tuple(
                    c.column_name,
                    c.data_type,
                    toBool(c.is_nullable),
                    toBool(sc.is_in_primary_key),
                    c.column_comment
                )
            ) AS "columns"
        FROM INFORMATION_SCHEMA.COLUMNS AS c
            LEFT JOIN system.columns AS sc ON sc.database = c.table_schema
            AND sc.table = c.table_name
            AND sc.name = c.column_name
        GROUP BY c.table_catalog,
            c.table_schema,
            c.table_name
    ) AS c ON t.table_catalog = c.table_catalog
    AND t.table_schema = c.table_schema
    AND t.table_name = c.table_name
WHERE t.table_catalog NOT IN (
        'system',
        'INFORMATION_SCHEMA',
        'information_schema'
    )
FORMAT TabSeparatedRaw;