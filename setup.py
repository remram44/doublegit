import io
import os
from setuptools import setup


# pip workaround
os.chdir(os.path.abspath(os.path.dirname(__file__)))


# Need to specify encoding for PY3, which has the worst unicode handling ever
with io.open('README.rst', encoding='utf-8') as fp:
    description = fp.read()
req = []
setup(name='doublegit',
      version='1.0',
      py_modules=['doublegit'],
      entry_points={
          'console_scripts': [
              'doublegit = doublegit:main']},
      install_requires=req,
      description="Version and backup a Git repository",
      author="Remi Rampin",
      author_email='remirampin@gmail.com',
      maintainer="Remi Rampin",
      maintainer_email='remirampin@gmail.com',
      url='https://github.com/remram44/doublegit',
      project_urls={
          'Homepage': 'https://github.com/remram44/doublegit',
          'Say Thanks': 'https://saythanks.io/to/remram44',
          'Source': 'https://github.com/remram44/doublegit',
          'Tracker': 'https://github.com/remram44/doublegit/issues',
      },
      long_description=description,
      license='MIT',
      keywords=['git', 'versioning', 'version control', 'backup'],
      classifiers=[
          'Development Status :: 4 - Beta',
          'Environment :: Console',
          'Intended Audience :: Developers',
          'Intended Audience :: System Administrators',
          'License :: OSI Approved :: MIT License',
          'Operating System :: OS Independent',
          'Topic :: Software Development :: Version Control :: Git',
          'Topic :: System :: Archiving :: Backup',
          'Topic :: Utilities'])
