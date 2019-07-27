import sqlite3
from datetime import datetime
import logging
import os
from os.path import join
import shutil
import subprocess
from subprocess import check_call
import tempfile
import unittest

import doublegit
from doublegit import Ref


logging.basicConfig(level=logging.INFO)


class TestFetch(unittest.TestCase):
    def test_parsing(self):
        output = b'''\
Fetching origin
remote: Enumerating objects: 14, done.
remote: Counting objects: 100% (14/14), done.
remote: Compressing objects: 100% (11/11), done.
remote: Total 14 (delta 3), reused 12 (delta 1), pack-reused 0
Unpacking objects: 100% (14/14), done.
From github.com:remram44/doublegit
 * [new branch]      master     -> origin/master
   673b728..466e90b  devel      -> origin/devel
 - [deleted]         (none)     -> origin/old
'''
        new, changed, removed = doublegit.parse_fetch_output(output)
        self.assertEqual(new, [Ref('origin', 'master', False)])
        self.assertEqual(changed, [Ref('origin', 'devel', False)])
        self.assertEqual(removed, [Ref('origin', 'old', False)])

    @staticmethod
    def time(n):
        return datetime(2019, 3, 16, 17, n, 0)

    def test_update(self):
        test_dir = tempfile.mkdtemp(prefix='doublegit_test_')

        try:
            # The "remote" we'll be watching
            origin = join(test_dir, 'origin')
            os.mkdir(origin)
            check_call(['git', 'init'], cwd=origin)

            def write(contents):
                with open(join(origin, 'f'), 'w') as fp:
                    fp.write(contents)
                check_call(['git', 'add', 'f'], cwd=origin)

            def env(n):
                time = self.time(n).strftime('%Y-%m-%d %H:%M:%S -0400')
                return {'GIT_COMMITTER_DATE': time,
                        'GIT_AUTHOR_DATE': time,
                        'GIT_AUTHOR_NAME': 'doublegit',
                        'GIT_AUTHOR_EMAIL': 'doublegit@example.com',
                        'GIT_COMMITTER_NAME': 'doublegit',
                        'GIT_COMMITTER_EMAIL': 'doublegit@example.com'}

            def commit(n, msg):
                time = self.time(n).strftime('%Y-%m-%d %H:%M:%S -0400')
                check_call(
                    ['git', 'commit', '--date=%s' % time, '-m', msg],
                    cwd=origin,
                    env=env(n),
                )

            # Recording folder
            mirror = join(test_dir, 'mirror')
            os.mkdir(mirror)
            check_call(['git', 'init', '--bare'], cwd=mirror)
            with open(join(mirror, 'config'), 'w') as fp:
                fp.write('[core]\n'
                         '\trepositoryformatversion = 0\n'
                         '\tfilemode = true\n'
                         '\tbare = true\n'
                         '\tlogallrefupdates = false\n'
                         '[remote "origin"]\n'
                         '\turl = ../origin\n'
                         '\tfetch = +refs/heads/*:refs/remotes/origin/*\n')
            self.assertFalse(os.path.exists(
                join(mirror, 'gitarchive.sqlite3')
            ))

            # New branch 'br1'
            check_call(['git', 'checkout', '-b', 'br1'], cwd=origin)
            write('one')
            commit(0, 'one')
            doublegit.update(mirror, time=self.time(1))
            self.assertTrue(os.path.exists(
                join(mirror, 'gitarchive.sqlite3')
            ))
            self.check_db(mirror, [
                ('br1', 1, None, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
            ])
            self.check_refs(mirror, {
                'keep-ae79568054d9fa2e4956968310655e9bcbd60e2f',
            })

            # Update branch br1
            write('two')
            commit(2, 'two')
            doublegit.update(mirror, time=self.time(3))
            self.check_db(mirror, [
                ('br1', 1, 3, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('br1', 3, None, '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'),
            ])
            self.check_refs(mirror, {
                'keep-8dcda34bbae83d2e3d856cc5dbc356ee6e947619',
            })

            # Force-push branch br1 back
            check_call(['git', 'reset', '--keep', 'ae79568'], cwd=origin)
            doublegit.update(mirror, time=self.time(4))
            self.check_db(mirror, [
                ('br1', 1, 3, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('br1', 3, 4, '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'),
                ('br1', 4, None, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
            ])
            self.check_refs(mirror, {
                'keep-8dcda34bbae83d2e3d856cc5dbc356ee6e947619',
            })

            # Delete branch br1, create br2
            check_call(['git', 'checkout', '-b', 'br2'], cwd=origin)
            check_call(['git', 'branch', '-D', 'br1'], cwd=origin)
            write('three')
            commit(5, 'three')
            doublegit.update(mirror, time=self.time(6))
            self.check_db(mirror, [
                ('br1', 1, 3, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('br1', 3, 4, '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'),
                ('br1', 4, 6, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('br2', 6, None, '54356c0e8c1cb663294d64157f517f980e5fbd98'),
            ])
            self.check_refs(mirror, {
                'keep-8dcda34bbae83d2e3d856cc5dbc356ee6e947619',
                'keep-54356c0e8c1cb663294d64157f517f980e5fbd98',
            })

            # Create light-weight tag1
            check_call(['git', 'tag', 'tag1',
                        'ae79568054d9fa2e4956968310655e9bcbd60e2f'],
                       cwd=origin)
            doublegit.update(mirror, time=self.time(7))
            self.check_db(mirror, [
                ('tag1', 7, None, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
            ], tags=True)

            # Create annotated tag2
            check_call(['git', 'tag', '-a', 'tag2', '-m', 'tag2msg',
                        '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'],
                       cwd=origin, env=env(8))
            doublegit.update(mirror, time=self.time(8))
            self.check_db(mirror, [
                ('tag1', 7, None, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('tag2', 8, None, '8fda1c0cfb4957e376fba4b53bf3ce080e25300c'),
            ], tags=True)

            # Move the tags
            check_call(['git', 'tag', '-f', 'tag1',
                        '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'],
                       cwd=origin)
            check_call(['git', 'tag', '-a', '-f', 'tag2', '-m', 'tag2msg',
                        'ae79568054d9fa2e4956968310655e9bcbd60e2f'],
                       cwd=origin, env=env(9))
            doublegit.update(mirror, time=self.time(9))
            self.check_db(mirror, [
                ('tag1', 7, 9, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('tag2', 8, 9, '8fda1c0cfb4957e376fba4b53bf3ce080e25300c'),
                ('tag1', 9, None, '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'),
                ('tag2', 9, None, 'a64697beb90c35d198fd25f2985cbc9e1ac1783e'),
            ], tags=True)

            # Remove the tags
            check_call(['git', 'tag', '-d', 'tag1', 'tag2'],
                       cwd=origin)
            doublegit.update(mirror, time=self.time(10))
            self.check_db(mirror, [
                ('tag1', 7, 9, 'ae79568054d9fa2e4956968310655e9bcbd60e2f'),
                ('tag2', 8, 9, '8fda1c0cfb4957e376fba4b53bf3ce080e25300c'),
                ('tag1', 9, 10, '8dcda34bbae83d2e3d856cc5dbc356ee6e947619'),
                ('tag2', 9, 10, 'a64697beb90c35d198fd25f2985cbc9e1ac1783e'),
            ], tags=True)
        finally:
            shutil.rmtree(test_dir)

    def check_db(self, repo, expected, tags=False):
        strftime = lambda n: (self.time(n).strftime('%Y-%m-%d %H:%M:%S')
                              if n else None)
        expected = [
            (name,
             strftime(from_date),
             strftime(to_date),
             sha)
            for name, from_date, to_date, sha in expected
        ]
        conn = sqlite3.connect(join(repo, 'gitarchive.sqlite3'))
        refs = list(conn.execute(
            '''
            SELECT name, from_date, to_date, sha
            FROM refs
            WHERE tag=?
            ORDER BY from_date, name;
            ''',
            [tags],
        ))
        self.assertEqual(refs, expected)

    def check_refs(self, repo, expected):
        out = subprocess.check_output(['git', 'branch'], cwd=repo)
        refs = {ref.decode('utf-8').strip() for ref in out.splitlines()}
        self.assertEqual(refs, expected)


if __name__ == '__main__':
    unittest.main()
