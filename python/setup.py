import setuptools

setuptools.setup(
    name="igrepper",
    version="0.0.2",
    author="Gustav Larsson",
    author_email="gustav.e.larsson@gmail.com",
    description="Interactive curses-based grepping tool",
    long_description="Interactive curses-based grepping tool",
    long_description_content_type="text/markdown",
    scripts=['bin/igrepper'],
    url="https://github.com/igoyak/igrepper",
    packages=setuptools.find_packages(),
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
)
