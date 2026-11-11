package io.github.dantengs.niumaclock.core

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNamingStrategy
import java.io.File

/** JSON 读写。格式与电脑版一致：字段蛇形命名、缺省值可省略、未知字段忽略。 */
@OptIn(ExperimentalSerializationApi::class)
object Store {
    val json = Json {
        namingStrategy = JsonNamingStrategy.SnakeCase
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = true
        coerceInputValues = true
        prettyPrint = true
        allowSpecialFloatingPointValues = true
    }

    fun decodeConfig(text: String): Config = json.decodeFromString(Config.serializer(), text)
    fun decodeData(text: String): Data = json.decodeFromString(Data.serializer(), text)
    fun encode(cfg: Config): String = json.encodeToString(Config.serializer(), cfg)
    fun encode(data: Data): String = json.encodeToString(Data.serializer(), data)

    /** 读文件；不存在时返回 null。解析失败时把坏文件改名备份，返回 null，绝不覆盖用户数据。 */
    fun <T> load(file: File, decode: (String) -> T): T? {
        if (!file.exists()) return null
        return try {
            decode(file.readText())
        } catch (e: Exception) {
            val backup = File(file.parentFile, "${file.name}.broken-${System.currentTimeMillis()}")
            file.renameTo(backup)
            null
        }
    }

    /** 原子写入：先写临时文件再改名。 */
    fun save(file: File, text: String) {
        file.parentFile?.mkdirs()
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) {
            file.delete()
            tmp.renameTo(file)
        }
    }
}
