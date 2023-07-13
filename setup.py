from setuptools import setup
from setuptools_rust import RustBin
from wheel.bdist_wheel import bdist_wheel as _bdist_wheel

with open('Cargo.toml') as f:
    for line in f:
        if line.startswith('version = "'):
            _, VERSION, _ = line.split('"')
            break


class bdist_wheel(_bdist_wheel):
    def finalize_options(self):
        super().finalize_options()
        self.root_is_pure = False

    def get_tag(self):
        _, _, plat = super().get_tag()
        return 'py3', 'none', plat


setup(
    version=VERSION,
    rust_extensions=[RustBin("sentry-cli")],
    cmdclass={'bdist_wheel': bdist_wheel},
)
