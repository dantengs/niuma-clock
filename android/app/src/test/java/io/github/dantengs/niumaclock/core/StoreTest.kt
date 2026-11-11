package io.github.dantengs.niumaclock.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.nio.file.Files
import java.time.LocalDate
import java.time.LocalDateTime

/** 电脑版（Rust serde）写出的 JSON 能被手机版读入。样例数据是编的。 */
class StoreTest {
    private val desktopConfig = """
        {
          "salary": {"monthly": 8000.0, "paid_days": 21.75, "work_start": "09:30:00", "work_end": "18:30:00",
                     "lunch_unpaid": true, "lunch_start": "12:00:00", "lunch_end": "13:30:00",
                     "tick_unit": "Minute", "overtime_multiplier": 0.0, "payday": 10, "currency": "¥"},
          "reminders": [{"id": 1, "title": "喝口水", "note": "", "interval_min": 40, "auto_start": true}],
          "meds": [
            {"id": 5, "name": "维C", "dose": "1 片", "note": "", "schedule": {"Times": {"times": ["08:00:00", "20:00:00"]}}},
            {"id": 6, "name": "止咳", "dose": "", "note": "", "schedule": {"Interval": {"minutes": 240}}}
          ],
          "alert_style": "Both", "sound": true, "snooze_minutes": 10, "pause_when_away": true,
          "work_end_reminder": false, "lunch_quiet": true, "floating_bar": true, "floating_pos": [100.0, 200.0],
          "hide_money": false, "boss_key": true, "effect_enabled": true, "effect_seconds": 7.0, "theme": "Dark",
          "colors_light": {"accent": [10, 20, 30], "gold": null, "hero": null, "bg": null, "card": null},
          "colors_dark": {}, "tray_hint_shown": true
        }
    """.trimIndent()

    private val desktopData = """
        {
          "next_id": 12,
          "sessions": [
            {"start": "2026-11-03T09:28:41.123456789", "end": "2026-11-03T18:40:02.5", "paid_secs": 28880.0,
             "overtime_secs": 0.0, "income": 371.2},
            {"start": "2026-11-04T09:31:00", "end": null, "paid_secs": 0.0, "overtime_secs": 0.0, "income": 0.0}
          ],
          "days": {"2026-11-03": {"done": 3, "skipped": 1, "meds_taken": 0, "health": "SubHealthy", "mood": null}},
          "loops": [{"id": 11, "template_id": 1, "title": "喝口水", "note": "", "interval_min": 40,
                     "next_fire": "2026-11-04T10:11:00", "paused_left": null, "pending_since": null, "done": 0, "skipped": 0}],
          "meds": [],
          "health": "SubHealthy", "mood": "Tired",
          "status_log": [{"at": "2026-11-03T10:00:00", "change": {"Health": "SubHealthy"}},
                         {"at": "2026-11-03T11:00:00", "change": {"Mood": "Tired"}}],
          "work_end_notified": "2026-11-03", "work_end_pending": false, "work_end_snooze_until": null,
          "sick_prompt_pending": false, "forgot_ack": null, "last_seen": "2026-11-04T10:00:00",
          "month_adjust": {"2026-11": 1200.5},
          "expenses": [{"id": 9, "at": "2026-11-03T12:10:00", "amount": 22.5, "category": "餐饮", "note": "面"}],
          "lunch_manual_until": null, "lunch_auto_off": null
        }
    """.trimIndent()

    @Test fun readsDesktopConfig() {
        val cfg = Store.decodeConfig(desktopConfig)
        assertEquals(8000.0, cfg.salary.monthly, 0.0)
        assertEquals(hm(13, 30), cfg.salary.lunchEnd)
        assertEquals(TickUnit.Minute, cfg.salary.tickUnit)
        assertEquals(listOf(hm(8, 0), hm(20, 0)), cfg.meds[0].schedule.times!!.times)
        assertEquals(240, cfg.meds[1].schedule.interval!!.minutes)
        assertEquals(AlertStyle.Both, cfg.alertStyle)
        assertEquals(ThemeMode.Dark, cfg.theme)
        assertEquals(listOf(10, 20, 30), cfg.colorsLight.accent)
        assertEquals(false, cfg.workEndReminder)
    }

    @Test fun readsDesktopData() {
        val data = Store.decodeData(desktopData)
        assertEquals(2, data.sessions.size)
        assertEquals(LocalDateTime.of(2026, 11, 3, 9, 28, 41, 123456789), data.sessions[0].start)
        assertNull(data.sessions[1].end)
        assertEquals(3, data.days[LocalDate.of(2026, 11, 3)]!!.done)
        assertEquals(Health.SubHealthy, data.statusLog[0].change.health)
        assertEquals(Mood.Tired, data.statusLog[1].change.mood)
        assertEquals(1200.5, data.monthAdjust["2026-11"]!!, 0.0)
        assertEquals("面", data.expenses[0].note)
        val e = Engine(Store.decodeConfig(desktopConfig), data)
        assertTrue("导入后进行中的打卡继续算", e.onDuty())
        assertEquals(12L, e.data.nextId)
    }

    @Test fun roundTripKeepsEverything() {
        val cfg = Store.decodeConfig(desktopConfig)
        val data = Store.decodeData(desktopData)
        assertEquals(cfg, Store.decodeConfig(Store.encode(cfg)))
        assertEquals(data, Store.decodeData(Store.encode(data)))
        val text = Store.encode(cfg)
        assertTrue("与电脑版同形", text.contains("\"Times\"") && text.contains("\"work_start\": \"09:30:00\""))
    }

    @Test fun brokenFileIsBackedUpNotOverwritten() {
        val dir = Files.createTempDirectory("niuma").toFile()
        val file = File(dir, "data.json").apply { writeText("{ not json") }
        assertNull(Store.load(file, Store::decodeData))
        assertTrue(dir.listFiles()!!.any { it.name.startsWith("data.json.broken-") })
        Store.save(file, Store.encode(Data()))
        assertEquals(Data().nextId, Store.load(file, Store::decodeData)!!.nextId)
        dir.deleteRecursively()
    }
}
