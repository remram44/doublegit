import argparse
import collections
from datetime import datetime
import itertools
import logging
import os
import re
import sqlite3
import subprocess


__version__ = '1.2'


logger = logging.getLogger(__name__)


_re_fetch = re.compile(r' ([+t*! -]) +([^ ]+|\[[^\]]+\]) +'
                       r'([^ ]+) +-> +([^ ]+)(?: +(.+))?$')


Ref = collections.namedtuple('Ref', ['remote', 'name', 'tag'])


class Operation(object):
    FAST_FORWARD = ' '
    FORCED = '+'
    PRUNED = '-'
    TAG = 't'
    NEW = '*'
    REJECT = '!'
    NOOP = '='


def fetch(repository):
    cmd = ['git', 'fetch', '--prune', 'origin',
           '+refs/tags/*:refs/tags/*', '+refs/heads/*:refs/remotes/origin/*']
    proc = subprocess.Popen(cmd, cwd=repository,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    out, _ = proc.communicate()
    if proc.wait() != 0:
        raise subprocess.CalledProcessError(proc.returncode, cmd)

    return parse_fetch_output(out)


def parse_remote_ref(ref, remote):
    remote_part, name = ref.split('/', 1)
    try:
        assert remote_part == remote
    except AssertionError:
        logger.error("remote=%r remote_part=%r", remote, remote_part)
        raise
    return Ref(remote, name, False)


def ref_name(ref):
    if ref.tag:
        return ref.name
    else:
        return '%s/%s' % (ref.remote, ref.name)


def parse_fetch_output(err):
    remote = 'origin'
    new = []
    changed = []
    removed = []
    for line in err.splitlines():
        line = line.decode('utf-8')
        m = _re_fetch.match(line)
        if m is not None:
            logger.info("> %s", line)
            op, summary, from_, to, reason = m.groups()

            if op == Operation.NEW:
                if '/' not in to:  # tag
                    new.append(Ref(remote, to, True))
                else:
                    new.append(parse_remote_ref(to, remote))
            elif op in (Operation.FAST_FORWARD, Operation.FORCED):
                changed.append(parse_remote_ref(to, remote))
            elif op == Operation.PRUNED:
                if '/' not in to:  # tag
                    removed.append(Ref(remote, to, True))
                else:
                    removed.append(parse_remote_ref(to, remote))
            elif op == Operation.TAG:
                changed.append(Ref(remote, to, True))
            elif op == Operation.REJECT:
                raise ValueError("Error updating ref %s" % to)
            else:
                raise RuntimeError
        else:
            logger.info("! %s", line)
    return new, changed, removed


def get_sha(repository, ref):
    cmd = ['git', 'rev-parse', ref]
    sha = subprocess.check_output(cmd, cwd=repository)
    return sha.decode('utf-8').strip()


def make_branch(repository, name, sha):
    cmd = ['git', 'branch', '-f', name, sha]
    subprocess.check_call(cmd, cwd=repository)


def included_branches(repository, target):
    cmd = ['git', 'branch', '--merged', target]
    out = subprocess.check_output(cmd, cwd=repository)
    refs = []
    for line in out.splitlines():
        refs.append(line.decode('utf-8').strip())
    return refs


def including_branches(repository, target):
    cmd = ['git', 'branch', '--contains', target]
    out = subprocess.check_output(cmd, cwd=repository)
    refs = []
    for line in out.splitlines():
        refs.append(line.decode('utf-8').strip())
    return refs


def delete_branch(repository, ref):
    cmd = ['git', 'branch', '-D', ref]
    subprocess.check_call(cmd, cwd=repository)


def update(repository, time=None):
    # Check Git repository (bare)
    if (not os.path.exists(os.path.join(repository, 'refs')) or
            not os.path.exists(os.path.join(repository, 'objects'))):
        raise ValueError("%s is not a Git repository" % repository)

    # Open database
    db_path = os.path.join(repository, 'gitarchive.sqlite3')
    if not os.path.exists(db_path):
        conn = sqlite3.connect(db_path)
        conn.execute(
            '''
            CREATE TABLE refs(
                remote TEXT NOT NULL,
                name TEXT NOT NULL,
                from_date DATETIME NOT NULL,
                to_date DATETIME NULL,
                sha TEXT NOT NULL,
                tag BOOLEAN NOT NULL
            );
            ''',
        )
    else:
        conn = sqlite3.connect(db_path)

    # Do fetch
    new, changed, removed = fetch(repository)

    if time is None:
        time = datetime.utcnow().strftime('%Y-%m-%d %H:%M:%S')

    # Update database
    for ref in itertools.chain(removed, changed):
        conn.execute(
            '''
            UPDATE refs SET to_date=?
            WHERE remote=? AND name=?
            ORDER BY from_date DESC
            LIMIT 1;
            ''',
            [time, ref.remote, ref.name],
        )
    for ref in itertools.chain(changed, new):
        sha = get_sha(repository, ref_name(ref))
        conn.execute(
            '''
            INSERT INTO refs(remote, name, from_date, to_date, sha, tag)
            VALUES(?, ?, ?, NULL, ?, ?);
            ''',
            [ref.remote, ref.name, time, sha, int(ref.tag)],
        )

    # Create refs to prevent garbage collection
    for ref in itertools.chain(changed, new):
        sha = get_sha(repository, ref_name(ref))
        make_branch(repository, 'keep-%s' % sha, sha)

    # Remove superfluous branches
    for ref in itertools.chain(changed, new):
        sha = get_sha(repository, ref_name(ref))
        keeper = 'keep-%s' % sha
        for br in included_branches(repository, sha):
            if br != keeper:
                delete_branch(repository, br)
        if not ref.tag and len(including_branches(repository, sha)) > 1:
            delete_branch(repository, keeper)

    conn.commit()


def main():
    logging.basicConfig(level=logging.INFO)

    parser = argparse.ArgumentParser()
    parser.add_argument('repository')

    args = parser.parse_args()
    update(args.repository)


if __name__ == '__main__':
    main()
