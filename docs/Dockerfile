FROM squidfunk/mkdocs-material:9
RUN pip install mkdocs-awesome-pages-plugin
RUN pip install pygments_ansi_color
RUN pip install "mkdocs-material[imaging]"
COPY tools/md_slash ${PACKAGES}/md_slash
RUN pip install -e ${PACKAGES}/md_slash
