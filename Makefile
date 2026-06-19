.PHONY: scaffold site verify clean lessons-count count

scaffold:
	python3 scripts/scaffold_course.py

site:
	node site/build.js

verify:
	@echo "Phases:"; find phases -mindepth 1 -maxdepth 1 -type d | wc -l
	@echo "Lessons (docs/en.md):"; find phases -name 'en.md' -path '*/docs/*' | wc -l
	@echo "Quizzes:"; find phases -name 'quiz.json' | wc -l
	@echo "Code stubs:"; find phases -path '*/code/*' -type f | wc -l

count: verify

clean:
	@echo "No build artifacts to clean (lessons are source)."

lessons-count:
	@find phases -name 'en.md' -path '*/docs/*' | wc -l
