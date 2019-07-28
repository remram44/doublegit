What is this?
=============

This is a small tool that does `git fetch` then writes down all ref updates in a SQLite3 database. It also creates refs to make sure no commit gets garbage collected.

What for?
=========

If the remote repository gets force-pushed to, some commits might get lost. There will also be no evidence in the Git log that such destructive changes happened. It is also possible for a commit to be created but pushed much later, or branches can get renamed; none of this appears in the Git log either.

This tool will allow you to get a clear picture of what happened, never lose a commit, and quickly get a snapshot of a repository at a specific point in time. It is therefore more suitable for backups than a mirror.

Doesn't the reflog do all that?
===============================

This is indeed similar to Git's built-in reflog, except that the reflog isn't aware of branch renames, and a branch's reflog gets deleted with the branch.

It is also local, so you wouldn't be able to read the reflog of a remote repository.

How to use
==========

First, create a bare repo::

```
$ mkdir my-repo-backup && cd my-repo-backup
$ git init --bare
```

Set up a remote `origin`:

```
$ git remote add origin https://github.com/my-name/my-repo.git
```

Then simply run doublegit once in a while:

```
$ doublegit /path/to/my-repo-backup
```

You can then query `gitarchive.sqlite3` for branch updates or for the position of the branches at a given point in time.

Next steps?
===========

I think it would be cool if this tool could record GitHub/GitLab/... API events too; things like issues/merge requests/comments.

It could also automatically backup all your starred repos.

Some kind of query interface needs to be written (I'm thinking web).
