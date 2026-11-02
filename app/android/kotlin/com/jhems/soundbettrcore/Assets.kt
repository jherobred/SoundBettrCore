package com.jhems.soundbettrcore

import android.content.Context
import android.os.Build
import java.io.File

/** sherpa-onnx needs real files, so bundled assets are unpacked once per app version. */
object Assets {
    fun copyFile(ctx: Context, asset: String, to: File) {
        to.parentFile?.mkdirs()
        val tmp = File(to.path + ".tmp")
        ctx.assets.open(asset).use { input -> tmp.outputStream().use { input.copyTo(it) } }
        tmp.renameTo(to)
    }

    /** Unpacks `kokoro/` to filesDir and returns its folder. Skips the work when this version already did it. */
    fun voice(ctx: Context): File {
        val dir = File(ctx.filesDir, "kokoro")
        val mark = File(dir, ".version")
        val version = versionCode(ctx).toString()
        if (mark.exists() && mark.readText() == version) return dir
        dir.deleteRecursively()
        copyDir(ctx, "kokoro", dir)
        mark.writeText(version)
        return dir
    }

    private fun copyDir(ctx: Context, asset: String, to: File) {
        val children = ctx.assets.list(asset).orEmpty()
        if (children.isEmpty()) {
            copyFile(ctx, asset, to)
            return
        }
        to.mkdirs()
        for (c in children) copyDir(ctx, "$asset/$c", File(to, c))
    }

    private fun versionCode(ctx: Context): Long {
        val info = ctx.packageManager.getPackageInfo(ctx.packageName, 0)
        return if (Build.VERSION.SDK_INT >= 28) info.longVersionCode else @Suppress("DEPRECATION") info.versionCode.toLong()
    }
}
