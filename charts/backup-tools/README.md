# backup-tools Helm Chart

The accompanying Helm chart for the [backup-tools application](../../app/backup-tools). 

This Helm chart will create:

* A `CronJob` invoking backup-tools at a configurable interval.
* A `Role` providing access to the Kubernetes API for scaling a given `Deployment`.
* A `ServiceAccount`, which may be disabled in favor of making one's own.
* A `RoleBinding` for the aforementioned `Role` and `ServiceAccount` (is not created if the `ServiceAccount` is not 
  created by this chart.)
* A `PersistentVolumeClaim` for the destination directory for the backups.

By default, this Helm chart:

* Creates the `ServiceAccount` and `RoleBinding`.
* Sets the cron job for a schedule of `10 3 * * 3`.
* Drops all capabilities, sets a read-only root file system, runs as non-root, and runs as user and group `1029:1029`.
* Copies data from a `/source` `emptyDir` volume to a `/destination` volume to be configured by the user.
* Maintains up to five backups at a time, rotating out older backups as new ones are created.
* Opts in to scaling a `Deployment`.
* Disables database backups.
* Configures an incremental backup.
* Keeps `Job` instances around for 7 days (`ttlSecondsAfterFinished`).


## Configuration

Configuration is performed by providing one's own `values.yaml` file for the following configuration elements.

### `image`

* `tag`: Overrides the default tag, `latest`, set in the chart.

### `serviceAccount`

* `create`: If `true`, creates a `ServiceAccount`.
* `annotations`: Annotations to provide the `ServiceAccount`.
* `name`: Name of the service account; will be generated if not provided.

### `role`

* `name`: An optional name to provide the `Role`; will be generated if not provided.

### `cronJob`

* `enabled`: If `false`, the `CronJob` resource will not be deployed. Defaults to `true`.
* `suspend`: Configures the `spec.suspend` option of the `CronJob`. Defaults to `false`.
* `schedule`: A cron expression for how often the `Job` should be ran.
* `concurrencyPolicy`: Configures the [`concurrencyPolicy`](https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/#concurrency-policy) 
  of the `CronJob`. Defaults to `Replace`.
* `startingDeadlineSeconds`: Configures the [`startingDeadlineSeconds`](https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/#starting-deadline) 
  of the `CronJob`. Defaults to `60` seconds.
* `ttlSecondsAfterFinished`: The time-to-live for finished `Job` resources; `Job` resources older than this value are 
   automatically cleaned up. Read the [documentation](https://kubernetes.io/docs/concepts/workloads/controllers/ttlafterfinished/) 
   for more information. Defaults to 7 days.

### `podSecurityContext`

Allows for setting any valid option for `podSecurityContext`. Defaults:

* `fsGroup`: Configures the file system group; defaults to `100`.

### `securityContext`

Allows for setting any valid option for `securityContext`. Defaults:

* `capabilities`: Drop `ALL`.
* `readOnlyRootFilesystem`: `true`
* `runAsNoRoot`: `true`
* `runAsUser`: `1029`
* `runAsGroup`: `1029`

### `env`

Configures the environment variables passed to the application, which is the primary mode of configuration for the 
application. Refer to the [`README.md` for the application](../../app/backup-tools/README.md) for more information on 
these configuration options.

*Note:* Some values in `env.config.postgres` and `env.config.mongo` optionally allow one to specify a secret name and 
a key within that secret to retrieve those values.

*Note:* `env.config.k8s`. `env.config.mongo`, and `env.config.postgres` may be left to their defaults if
`env.config.app.scaleDeploymentEnabled`, `env.config.app.mongoBackupEnabled`, and `env.config.app.postgresBackupEnabled` 
are respectively set to `false`.

*Note:* `env.config.app.sourcePath` is mounted as an `emptyDir` volume into the container. It is expected that the 
application can write to this directory as it will write the database backup(s) here prior to any file backups.

```yaml
env:
  config:
    app:
      backupType: "INCREMENTAL"
      backupName: "Backup"
      sourcePath: "/source"
      destinationPath: "/destination"
      maxNumberOfBackups: 5
      scaleDeploymentEnabled: true
      postgresBackupEnabled: false
      mongoBackupEnabled: false
      rustBacktrace: 1
      rustLog: "info"
    compressed:
      excludeFilePath: ""
      timeout: 3600
    incremental:
      destinationOwner: ""
      destinationGroup: ""
      excludeFilePath: ""
      timeout: 300 # seconds == 5 minutes
      wholeFile: false
    k8s:
      cacrtPath: "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt"
      tokenPath: "/var/run/secrets/kubernetes.io/serviceaccount/token"
      serviceNamespace: ""
      namespaceFile: "/var/run/secrets/kubernetes.io/serviceaccount/namespace"
      serviceDeploymentName: ""
      workloadType: "DEPLOYMENT"
    mongo:
      host: ""
      hostSecret: {}
      #     name: ""
      #     key: ""
      port: ""
      username: ""
      usernameSecret: {}
      #     name: ""
      #     key: ""
      configurationPath: "" # "/config/mongo/config.yaml"
      databaseName: ""
      databaseNameSecret: {}
      #     name: ""
      #     key: ""
      authDatabaseName: ""
      authDatabaseNameSecret: {}
      #     name: ""
      #     key: ""
      authMechanism: ""
      collection: ""
      collectionSecret: {}
      #     name: ""
      #     key: ""
      queryPath: "" # "/config/mongo/query.json"
    postgres:
      host: ""
      hostSecret: {}
      #     name: ""
      #     key: ""
      port: ""
      databaseName: ""
      databaseNameSecret: {}
      #     name: ""
      #     key: ""
      username: ""
      usernameSecret: {}
      #     name: ""
      #     key: ""
      password: ""
      passwordSecret: {}
      #     name: ""
      #     key: ""
      url: "" # postgres://username:password@host:port
      urlSecret: {}
      #      name: ""
      #      key: ""
```

### `volume`

Configures the volumes to copy data from (source) and copy data to (destination).

#### `sources`

An array of source volumes to copy from. Each element has three properties:

* `name`: The name of the volume.
* `claimName`: The existing name of the `PersistentVolumeClaim` to copy from.
* `mountPath`: Where the volume should be mounted to inside the container; in order for the volume's contents to be 
  included in a backup, the volume _must_ be mounted as a subdirectory of `env.config.app.sourcePath`. For example, if 
  the default source path `/source` is used, then the `mountPath` must be `/source/my_volume`.
* `readOnlyPvc`: Value of `readOnly` on the PVC. Defaults to `false`.
* `readOnly`: Value of `readOnly` on the volume mount. Defaults to `false`.

#### `destination`

Configures the destination volume where backup-tools will save data copied from the source volumes.

* `claimName`: Name of the `PersistentVolumeClaim` to use for the destination files.
* `createPvc`: If `true`, will create the `PersistentVolumeClaim` automatically if it does not exist. Defaults to `false`.
* `annotations`: Annotations to assign to a created PVC.
* `spec`: `spec` of a created PVC.

*Note:* It is expected that backup-tools will have complete ownership over this volume; files may be deleted out of this 
volume as a part of the backup rotation governed by `env.config.app.maxNumberOfBackups`.

### `extraVolumes`

An array of additional volumes to mount into the container. This is intended for including additional configuration 
and other helpful files (e.g. excludes file for `rsync`) into the container.

Each item must have the following properties:

* `name`: The name to assign to the volume.
* `mountPath`: The location within the container to mount the volume.
* `claim`: The claim information for the volume.
