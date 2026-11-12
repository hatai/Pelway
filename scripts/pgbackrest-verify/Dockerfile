# pgBackRest 検証用イメージ（rootless Podman でのビルド・実行を前提）。
# pgBackRest は公式の配布イメージが存在しないため、postgres:16-alpine の上に
# リリース tarball（sha256sum ファイルで検証）から meson でビルドする。
# 出典: https://pgbackrest.org / https://github.com/pgbackrest/pgbackrest
# 必要ライブラリは pgBackRest 2.60.0 README.md 参照（libpq・OpenSSL・libxml2・
# lz4・bz2・zlib が必須、zstd を追加）。
FROM docker.io/library/postgres:16-alpine

ARG PBGR_VERSION=2.60.0
# release/2.60.0 の pgbackrest-2.60.0.tar.gz の sha256。
# 出典: https://github.com/pgbackrest/pgbackrest/releases/download/release/2.60.0/pgbackrest-2.60.0.tar.gz.sha256sum
ARG PBGR_SHA256=6d3ee1b5defdadc85228969b4be804096f5e65e54707bb95c7c4c4ce41f5d007

RUN apk add --no-cache --virtual=.build-dependencies \
      build-base meson ninja pkgconf libpq-dev libxml2-dev openssl-dev \
      zlib-dev lz4-dev zstd-dev bzip2-dev \
 && wget -q "https://github.com/pgbackrest/pgbackrest/releases/download/release/${PBGR_VERSION}/pgbackrest-${PBGR_VERSION}.tar.gz" \
 && echo "${PBGR_SHA256}  pgbackrest-${PBGR_VERSION}.tar.gz" | sha256sum -c - \
 && tar xf "pgbackrest-${PBGR_VERSION}.tar.gz" \
 && meson setup "build" "pgbackrest-${PBGR_VERSION}" \
 && ninja -C build install \
 && rm -rf "pgbackrest-${PBGR_VERSION}" "pgbackrest-${PBGR_VERSION}.tar.gz" build \
 && apk del .build-dependencies

# 実行時に必要な依存は、ビルド依存の削除より後に入れる（apk del は仮想
# グループに入っていた依存をまとめて消すため、先に入れておくと libbz2 が
# 消える）。libfaketime は保持ポリシー（time 型）の検証でバックアップの
# 作成時刻を偽装するのに使う。
RUN apk add --no-cache libfaketime bzip2 libbz2
