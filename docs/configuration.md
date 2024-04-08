# Configuration

## Initializing & Updating a Configuration Directory

The connector requires a configuration directory to run

When part of a ddn project, this configuration directory lives in the project.

### Using the hasura ddn cli

Hasura DDN CLI will initialize and update the configuration directory for you

Follow the instructions on how to add a connector


### Using the ddn-clickhouse-cli executable

You can run the ndc-clickhouse-cli executable yourself


If you have the executable:
```
ndc-clickhouse-cli.exe --connector-context-path ./config --clickhouse-url "URL" --clickhouse-username "USERNAME" --clickhouse-password "PASSWORD" update
```

If you have the source code:

```
cargo run --package ndc-clickhouse-cli -- --connector-context-path ./config --clickhouse-url "URL" --clickhouse-username "USERNAME" --clickhouse-password "PASSWORD" update
```

See also: [development instructions](./development.md)

## Tables

Tables are added by introspecting the database provided during init/update of the configuration directory.
Most users do not need to further alter this configuration, but there are a couple additional options

### Table alias

The keys in the tables object in the configuration file can be changed to modify the alias a table will be exposed under.

This alias must remain unique

### Table Return Type

Tables can return the same type as another table.

This is useful for views that return rows from another table.

This will allow both tables to share an object type,
which in turn allows both tables to share relationships and object type permissions.

## Native Queries

This connector supports native queries: writing raw SQL queries to treat as collections (virtual tables)

This is an alternative to writing views on the database, which is usually preferable, but may not be plausible.
This can also be useful to iterate on views before creating them on the database.

You can write a native query as a `.sql` file in your configuration directory, typically in a dedicated subdirectory

Your file may only contain a single statement.

Arguments may be specified using the [clickhouse parameter syntax](https://clickhouse.com/docs/en/interfaces/cli#cli-queries-with-parameters-syntax)

```sql
-- queries/ArtistByName.sql
SELECT *
FROM "default"."Artist"
WHERE "Artist"."Name" = {ArtistName: String}
```

Then add the query to your `configuration.json` file.
You'll need to figure out the query return type

```json
{
    "tables": {},
    "queries": {
        "Name": {
            "exposed_as": "collection",
            "file": "queries/ArtistByName.sql",
            "return_type": {
                "kind": "definition",
                "columns": {
                    "ArtistId": "Int32",
                    "Name": "String"
                }
            }
        }
    }
}
```

To figure out your return type, you can use the [ClickHouse `toTypeName` function](https://clickhouse.com/docs/en/sql-reference/functions/other-functions#totypenamex)

One way to get the return types for your SQL statemen:

```sql
SELECT * APPLY toTypeName
FROM (
    -- your SQL here
) q LIMIT 1;

```

Alternatively, if your query returns the same type as another table, and you want this reflected in your schema:

```json
{
    "tables": {
        "Artist": {
            "name": "Artist",
            "schema": "default",
            "comment": "",
            "primary_key": {
                "name": "ArtistId",
                "columns": [
                "ArtistId"
                ]
            },
            "return_type": {
                "kind": "definition",
                "columns": {
                "ArtistId": "Int32",
                "Name": "Nullable(String)"
                }
            }
        }        
    },
    "queries": {
        "Name": {
            "exposed_as": "collection",
            "file": "queries/ArtistByName.sql",
            "return_type": {
                "kind": "table_reference",
                "table_name": "Artist"
            }
        }
    }
}
```
