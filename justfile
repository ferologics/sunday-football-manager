alias r := run
alias c := check

run:
  uv run streamlit run app.py

check:
  uv run ruff check app.py
  uv run ty check app.py
