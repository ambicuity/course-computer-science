-- =============================================================
-- Denormalized vs Normalized Schema — School Database
-- Phase 10, Lesson 04: Normalization 1NF → BCNF (and 4NF)
-- =============================================================

-- =============================================================
-- DENORMALIZED (violates 2NF, 3NF, BCNF)
-- One table packs everything: students, advisors, courses, grades.
--
-- Anomalies:
--   UPDATE: changing Kim's name to Kim-Chen touches N rows
--   INSERT: can't add a new advisor with zero students
--   DELETE: removing last student in CS101 loses course+instructor
-- =============================================================

CREATE TABLE enrollments_denormalized (
    student_id      INTEGER,
    student_name    TEXT    NOT NULL,
    major           TEXT,
    advisor_id      INTEGER,
    advisor_name    TEXT,
    course_code     TEXT,
    course_title    TEXT,
    instructor_id   INTEGER,
    instructor_name TEXT,
    grade           TEXT,
    PRIMARY KEY (student_id, course_code)
);

INSERT INTO enrollments_denormalized VALUES
    (1, 'Alice',   'CS',    10, 'Dr. Kim',   'CS101', 'Intro CS',    100, 'Prof. Jones', 'A'),
    (1, 'Alice',   'CS',    10, 'Dr. Kim',   'MATH200', 'Calc II',    101, 'Prof. Lee',  'B+'),
    (2, 'Bob',     'EE',    11, 'Dr. Chen',  'CS101', 'Intro CS',    100, 'Prof. Jones', 'B'),
    (3, 'Charlie', 'Math',  10, 'Dr. Kim',   'CS101', 'Intro CS',    100, 'Prof. Jones', 'A-'),
    (4, 'Diana',   'CS',    10, 'Dr. Kim',   'MATH200', 'Calc II',   101, 'Prof. Lee',  'A'),
    (5, 'Eve',     'EE',    11, 'Dr. Chen',  'EE201',  'Circuits',   102, 'Prof. Smith', 'B+');

-- UPDATE anomaly: Dr. Kim changes name — must update 4 rows
SELECT student_id, advisor_name
FROM enrollments_denormalized
WHERE advisor_id = 10;
-- → 4 rows all say "Dr. Kim", must update every one

-- INSERT anomaly: can't add a course without a student
INSERT INTO enrollments_denormalized
    (student_id, course_code, course_title, instructor_id, instructor_name)
VALUES (NULL, 'PHYS101', 'Physics I', 103, 'Prof. Brown');
-- → violates PRIMARY KEY (student_id is NULL)

-- DELETE anomaly: deleting Eve's enrollment loses EE201 entirely
DELETE FROM enrollments_denormalized WHERE student_id = 5 AND course_code = 'EE201';
SELECT * FROM enrollments_denormalized WHERE course_code = 'EE201';
-- → no rows; course EE201 no longer exists in the database

-- =============================================================
-- NORMALIZED (BCNF)
-- Every relation is in BCNF: each fact lives in one place.
-- =============================================================

CREATE TABLE advisors (
    advisor_id   INTEGER PRIMARY KEY,
    advisor_name TEXT NOT NULL
);

CREATE TABLE students (
    student_id  INTEGER PRIMARY KEY,
    student_name TEXT NOT NULL,
    major       TEXT,
    advisor_id  INTEGER REFERENCES advisors(advisor_id)
);

CREATE TABLE instructors (
    instructor_id   INTEGER PRIMARY KEY,
    instructor_name TEXT NOT NULL
);

CREATE TABLE courses (
    course_code   TEXT PRIMARY KEY,
    course_title  TEXT NOT NULL,
    instructor_id INTEGER REFERENCES instructors(instructor_id)
);

CREATE TABLE enrollments (
    student_id  INTEGER REFERENCES students(student_id),
    course_code TEXT REFERENCES courses(course_code),
    grade       TEXT,
    PRIMARY KEY (student_id, course_code)
);

INSERT INTO advisors VALUES
    (10, 'Dr. Kim'),
    (11, 'Dr. Chen');

INSERT INTO students VALUES
    (1, 'Alice',   'CS',   10),
    (2, 'Bob',     'EE',   11),
    (3, 'Charlie', 'Math', 10),
    (4, 'Diana',   'CS',   10),
    (5, 'Eve',     'EE',   11);

INSERT INTO instructors VALUES
    (100, 'Prof. Jones'),
    (101, 'Prof. Lee'),
    (102, 'Prof. Smith'),
    (103, 'Prof. Brown');

INSERT INTO courses VALUES
    ('CS101',  'Intro CS',   100),
    ('MATH200','Calc II',    101),
    ('CS201',  'Data Structs', 100),
    ('EE201',  'Circuits',   102),
    ('PHYS101','Physics I',  103);

INSERT INTO enrollments VALUES
    (1, 'CS101',  'A'),
    (1, 'MATH200','B+'),
    (2, 'CS101',  'B'),
    (3, 'CS101',  'A-'),
    (4, 'MATH200','A'),
    (5, 'EE201',  'B+');

-- =============================================================
-- Queries that become SIMPLER after normalization
-- =============================================================

-- Denormalized: must use DISTINCT or LIMIT 1 to find an advisor's name
SELECT DISTINCT advisor_name
FROM enrollments_denormalized
WHERE advisor_id = 10;

-- Normalized: direct lookup, no duplicates
SELECT a.advisor_name
FROM advisors a
WHERE a.advisor_id = 10;

-- Denormalized: student transcript requires single-table scan
SELECT student_name, course_code, grade
FROM enrollments_denormalized
WHERE student_id = 1;

-- Normalized: clean join, no data duplication
SELECT s.student_name, e.course_code, e.grade
FROM students s
JOIN enrollments e ON e.student_id = s.student_id
WHERE s.student_id = 1;

-- =============================================================
-- No more UPDATE anomaly: change advisor name, one row
UPDATE advisors SET advisor_name = 'Dr. Kim-Chen' WHERE advisor_id = 10;
SELECT * FROM advisors;
-- → exactly one row changed, every student sees the new name
-- Rollback to keep demo idempotent
ROLLBACK;

-- No more INSERT anomaly: add a course without students
INSERT INTO courses VALUES ('PHYS101', 'Physics I', 103);
SELECT * FROM courses WHERE course_code = 'PHYS101';
-- → course exists regardless of enrollments
DELETE FROM courses WHERE course_code = 'PHYS101';

-- No more DELETE anomaly: deleting an enrollment doesn't destroy course
DELETE FROM enrollments WHERE student_id = 5 AND course_code = 'EE201';
SELECT * FROM courses WHERE course_code = 'EE201';
-- → EE201 still exists in the courses table
-- Re-insert for consistency
INSERT INTO enrollments VALUES (5, 'EE201', 'B+');
