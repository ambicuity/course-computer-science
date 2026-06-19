-- DuckDB OLAP Examples: Columnar Storage, Parquet, Aggregations, Window Functions
-- Run: duckdb < main.sql

-- Meta: output mode
.mode column
.width 40 20

-- ── 1. In-memory OLAP: Create a star-schema-like table ──────────────────
CREATE TABLE sales AS
SELECT *
FROM (
    SELECT 'NA' AS region, 101 AS product_id, 10 AS quantity, 99.99 AS price, 0 AS discount
    UNION ALL SELECT 'NA', 102, 5, 45.50, 10
    UNION ALL SELECT 'EU', 101, 20, 99.99, 5
    UNION ALL SELECT 'EU', 103, 15, 200.00, 0
    UNION ALL SELECT 'APAC', 102, 8, 45.50, 15
    UNION ALL SELECT 'APAC', 101, 12, 99.99, 0
    UNION ALL SELECT 'NA', 103, 3, 200.00, 20
    UNION ALL SELECT 'EU', 104, 25, 12.99, 0
    UNION ALL SELECT 'LATAM', 102, 30, 45.50, 5
    UNION ALL SELECT 'APAC', 104, 18, 12.99, 10
    UNION ALL SELECT 'MEA', 101, 7, 99.99, 0
)
LIMIT 10;

ALTER TABLE sales ADD COLUMN revenue DOUBLE;
UPDATE sales SET revenue = quantity * price * (1 - discount / 100.0);

SELECT * FROM sales ORDER BY region;

-- ── 2. Aggregation Queries (OLAP core) ──────────────────────────────────
.echo on

-- Total revenue by region
SELECT region, COUNT(*) AS num_sales, SUM(revenue) AS total_revenue
FROM sales
GROUP BY region
ORDER BY total_revenue DESC;

-- Average discount by product
SELECT product_id, AVG(discount) AS avg_discount
FROM sales
GROUP BY product_id
ORDER BY product_id;

-- ── 3. Window Functions in OLAP ─────────────────────────────────────────

-- Running total of revenue per region (ordered by nothing — show all)
SELECT region, product_id, revenue,
       SUM(revenue) OVER (PARTITION BY region ORDER BY product_id) AS running_total
FROM sales;

-- Rank products by revenue within each region
SELECT region, product_id, revenue,
       RANK() OVER (PARTITION BY region ORDER BY revenue DESC) AS rank_in_region
FROM sales;

-- Moving average (2-row window)
SELECT region, product_id, revenue,
       AVG(revenue) OVER (ORDER BY product_id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_avg
FROM sales;

-- ── 4. EXPLAIN — See the Vectorized Plan ────────────────────────────────

EXPLAIN SELECT region, SUM(revenue) FROM sales WHERE revenue > 100 GROUP BY region;

-- ── 5. DuckDB Parquet Functions ─────────────────────────────────────────

-- Generate CSV and load it
COPY sales TO '/tmp/sales_demo.csv' (HEADER, DELIMITER ',');
SELECT COUNT(*) AS csv_rows FROM read_csv('/tmp/sales_demo.csv', header=true);

-- Write Parquet
COPY sales TO '/tmp/sales_demo.parquet' (FORMAT PARQUET);
SELECT 'Wrote Parquet' AS status;

-- Read Parquet with column projection (DuckDB automatically does page-level pruning)
SELECT region, SUM(revenue) AS total
FROM read_parquet('/tmp/sales_demo.parquet')
WHERE revenue > 50
GROUP BY region;

-- Read Parquet metadata (row groups, column chunks, statistics)
SELECT * FROM parquet_metadata('/tmp/sales_demo.parquet');

-- Read Parquet schema
SELECT * FROM parquet_schema('/tmp/sales_demo.parquet');

-- ── 6. Large-scale Ana: TPC-H style "total sales by region" ─────────────

-- Generate 100K rows via DuckDB's built-in generate_series
CREATE TABLE big_sales AS
SELECT
    CASE WHEN random() < 0.3 THEN 'NA'
         WHEN random() < 0.55 THEN 'EU'
         WHEN random() < 0.75 THEN 'APAC'
         WHEN random() < 0.9 THEN 'LATAM'
         ELSE 'MEA'
    END AS region,
    (random() * 999 + 1)::INT AS product_id,
    (random() * 49 + 1)::INT AS quantity,
    round(random() * 495 + 5, 2) AS price,
    CASE WHEN random() < 0.1 THEN 0 ELSE 10 * (random() * 2)::INT END AS discount
FROM generate_series(1, 100000);

ALTER TABLE big_sales ADD COLUMN revenue DOUBLE;
UPDATE big_sales SET revenue = quantity * price * (1 - discount / 100.0);

-- Analytical query (vectorized, columnar)
.timer on
SELECT region, COUNT(*) AS orders, SUM(revenue) AS total_revenue, AVG(revenue) AS avg_revenue
FROM big_sales
WHERE revenue > 100
GROUP BY region
ORDER BY total_revenue DESC;
.timer off

-- Window function over all data
SELECT product_id, COUNT(*) AS cnt, SUM(revenue) AS total,
       RANK() OVER (ORDER BY SUM(revenue) DESC) AS rank
FROM big_sales
GROUP BY product_id
ORDER BY rank
LIMIT 10;

-- Compare DuckDB's vectorized plan vs SQLite's row-at-a-time
EXPLAIN ANALYZE SELECT region, SUM(revenue) FROM big_sales GROUP BY region;
