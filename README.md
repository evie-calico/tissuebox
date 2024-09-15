# Tissue

Tissue is a simple note-taking application intended for programming.
Tissues are small, short term tasks that you can summarize in a single git commit.

Tissues are primarily meant to track and separate short-term tasks which can be tossed upon completion;
so long as each tissue is closed by a commit, git will keep a history of ocmpleted tasks on its own.
However, sometimes "small" issues turn out to be larger than initially expected,
and require long term archival such as a github issue to track progress over time.
In this case, tissues are designed to be trivial to promote into proper issues,
using the same basic storage format.

## tissuebox format

A tissue box file is represented in the TOML format as a table of tables, where each table's key is its title.

```toml
["Implement Foo"]
["Upgrade Bar"]
["Remove Baz"]
```

Each table may contain the following keys:

### tags

Assign a list of tags (represented as strings) to an issue. Used for categorization and search.

```toml
["Implement Foo"]
tags = ["High priority"]
```

### desc

Associates an arbitrary paragraph of notes with the tissue.

```toml
["Upgrade Bar"]
desc = "Relies on implementation of Foo"
```

