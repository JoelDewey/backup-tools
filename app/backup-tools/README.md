# backup-tools Application

This is the Rust-based application for backup-tools. It coordinates executing several subprocesses to perform the 
backup tasks:

1. First, it optionally scales down a Kubernetes `Deployment` to zero replicas.
2. It then optionally executes `pg_dump` to create a backup of a database.
3. It then performs a file copy, via `rsync`, from a source directory to a target directory.
4. It then scales up the aforementioned Kubernetes `Deployment` back to the original number of replicas, given 
   that it was scaled down in the first step.

Backups are saved as a new directory with a directory name taking the format `YYYY-mm-DD_HHMMSS_BackupName`
inside the target directory. These backups are creating using hard links to a previous backup if one exists to minimize 
space utilization. Backups optionally may be deleted automatically once a maximum number of backups is reached; if configured,
then after creating the latest backup the oldest backup is deleted.

## Prerequisites

backup-tools requires the following to be installed:

* Rust 1.73 or greater.
* `rsync`
* `pg_dump` (commonly in a `postgresl-client` package)

It is designed to be run via a container in a Kubernetes cluster and expects that a bearer token and certificate for 
working with a cluster's API to be present unless `SCALE_DEPLOYMENT_ENABLED` is set to false.


## Building

backup-tools currently targets Rust 1.73 and is intended to be compatible with musl.

Builds can be ran via `cargo`: `cargo build --release`

The [Dockerfile](Dockerfile) can also be built to generate a container image based off of Alpine Linux.


## Configuration

backup-tools supports various configuration options, provided as environment variables, to customize the backup process.

### General Application Configuration

* `BACKUP_NAME` (Required): Some string, safe for use in a file path, that is used in the final name of the directory
  created within the target directory (e.g. `YYYY-mm-DD_HHMMSS_BackupName`).
* `SOURCE_PATH` (Required): The path to the source directory that backup-tools will copy from to the destination path; 
  this path _must_ be writable as it will be used to store the database backup if needed.
* `DESTINATION_PATH` (Required): The path to the directory of backups. Backups will be added as subdirectories of this  
  directory. It is expected that this directory is for use _only_ by this application; other files may be treated as 
  older backups and may be deleted otherwise.
* `MAX_NUMBER_OF_BACKUPS` (Required): The maximum number of backups to keep; if creating a new backup would exceed this 
  number of backups, then the oldest backup is deleted after creating the new backup. Set to `0` to disable deleting 
  older backups.
* `SCALE_DEPLOYMENT_ENABLED`: If set to `true`, will scale down a target `Deployment` prior to performing backups and 
  then will scale that `Deployment` back up once the backup is made. Set to `false` to disable scaling.
* `POSTGRES_BACKUP_ENABLED`: If set to `true`, will execute `pg_dump` to backup a PostgreSQL database. Set to `false` to 
  disable backing up a PostgreSQL database.

### Kubernetes Configuration

These options configure the communication to the Kubernetes API made while scaling a `Deployment` to prevent the other 
application from modifying data while backup-tools copies it.

It is assumed that a `Role`, `RoleBinding`, and `ServiceAccount` are available for the application to use. The `Role` 
must provide access to the `get` and `patch` verbs on `apps` `Deployment` objects.

These settings are only utilized when `SCALE_DEPLOYMENT_ENABLED` is set to `true`.

* `KUBERNETES_TOKEN_PATH` (Required): The path to the bearer token file mounted into the container by Kubernetes.
* `KUBERNETES_CACRT_PATH` (Required): The path to the `ca.crt` certificate for `HTTPS` communication with the 
  Kubernetes API.
* `KUBERNETES_SERVICE_HOST` (Required): The host of the Kubernetes API; usually provided by Kubernetes automatically.
* `KUBERNETES_SERVICE_PORT_HTTPS` (Required): The port of the Kubernetes API; usually provided by Kubernetes automatically.
* `KUBERNETES_SERVICE_DEPLOYMENT_NAME` (Required): The name of the `Deployment` to scale.
* `KUBERNETES_SERVICE_NAMESPACE`: The namespace of the `Deployment` to scale; if not provided, backup-tools will read 
  from the `namespace` file mounted into the container by Kubernetes.
* `KUBERNETES_NAMESPACE_FILE_PATH`: The path to the `namespace` file mounted into the container by Kubernetes. Only
  required when `KUBERNETES_SERVICE_NAMESPACE` is not set.

### PostgreSQL Backup Configuration

These configuration options modify how the PostgreSQL backup, using `pg_dump`, is performed. These options are only 
utilized when `POSTGRES_BACKUP_ENABLED` is set to `true`.

There are two ways to configure the PostgreSQL backup: 

1. Specifying host, port, etc. of the database individually or;
2. Specifying a URL of the form `postgres://user:pass@host:port/dbname`.

#### Individual Configuration

* `POSTGRES_HOST` (Required): The hostname of the PostgreSQL server.
* `POSTGRES_USERNAME` (Required): The user to log into the server as.
* `POSTGRES_PASSWORD` (Required): The password for the aforementioned PostgreSQL user.
* `POSTGRES_PORT`: The port of the PostgreSQL server; defaults to `5432` if not provided.
* `POSTGRES_DATABASE_NAME`: The name of the database to backup; defaults to allowing `pg_dump` to discover the database 
  name if not provided.

#### URL Configuration

* `POSTGRES_URL` (Required): A URL of the form `postgres://user:pass@host:port/dbname`. Port and database 
  name (`dbname`) are both optional and will default to the behavior described in the "Individual Configuration" section.

### Incremental File Backup Configuration

These options are used to configure `rsync` while making an incremental file backup.

* `INCR_TIMEOUT`: The amount of time, in seconds, to wait for `rsync` to complete before killing the process. Defaults 
  five minutes.
* `INCR_EXCLUDE_FILE_PATH`: The path to a file with patterns of files for `rsync` to exclude. Please refer to the 
  `rsync` `man` pages for details on the `--exclude-from=` option, which is what this variable configures. 
* `INCR_DESTINATION_OWNER`: The owner ID/name to use for the backup; passed directly as `rsync --chown=owner:group`. Requires 
  root access in the container.
* `INCR_DESTINATION_GROUP`: The group ID/name to use for the backup; passed directly as `rsync --chown=owner:group`. Requires
  root access in the container.