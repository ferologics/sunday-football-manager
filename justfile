alias c := check
alias r := run
alias s := setup


check:
  uv run ruff check app.py
  uv run ty check app.py

run:
  uv run streamlit run app.py

setup:
  uv sync
