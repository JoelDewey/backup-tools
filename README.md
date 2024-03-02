# backup-tools

An application to backup PostgreSQL databases and files for use within my home Kubernetes cluster.

* Automatically scales down a given Kubernetes workload (e.g. `Deployment`) prior to performing backups and will scale 
  it back to the original number of replicas after it has finished performing the backup operation.
* Can connect to a PostgreSQL server and make a backup using `pg_dump`.
* Can also connect to a MongoDB server and make a backup using `mongodump`. 
* Creates new backups using `rsync`, using hard links to save on storage usage.
* Can automatically rotate out older backups as newer ones are created.

The application itself is written using Rust and is deployable via a provided Helm chart. Please see 
[the documentation and code for the application](app/backup-tools) to learn more about how to build, run, and configure 
the application and then read through [the Helm chart and its documentation](charts/backup-tools) to learn more about 
how to deploy the application to a Kubernetes cluster.

## Restoring from Backup

backup-tools does not have any automated facility for restoring from backups. It is expected that an administrator 
restore file backups to the correct locations and, if necessary, execute the correct database restore tool using the 
database backups.

### PostgreSQL

To restore a PostgreSQL backup:

```bash
cd /path/to/directory/with/toc.dat
pg_restore -h hostname.local -p 5432 -U username --create -Fd -d database .
```

### MongoDB

To restore a MongoDB backup:

```bash
cd /path/to/directory/with/mongo.gz
mongorestore \
  --gzip \
  --archive=mongo.gz \ # '=' here is _required_!
  -u admin \
  --authenticationDatabase admin
```