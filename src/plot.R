library(ggplot2)
library(ggthemes)
library(patchwork)

my_theme = theme_excel_new() +
  theme(
    axis.text.y.right = element_blank(),
    axis.title.y.right = element_text(),
    panel.grid.minor.x = element_line(linetype = "dotted",
                                      colour = "#D9D9D9")
  )

x_scale <- scale_x_datetime(
  breaks = function(x) {
    seq.POSIXt(lubridate::floor_date(x[1], "day"),
               lubridate::ceiling_date(x[2], "day"),
               by = "1 day")
  },
  date_labels = "%b %d",
)

path <- commandArgs(trailingOnly = TRUE)[1]
measurements <-
  readr::read_tsv(
    path,
    col_names = FALSE,
    col_types = list(
      X1 = readr::col_datetime(format = "%Y-%m-%d %H:%M:%S %z"),
      X2 = readr::col_double(),
      X3 = readr::col_double()
    )
  )

temp_plot <- ggplot(measurements, aes(x = X1, y = X2)) +
  my_theme +
  geom_line(color = "firebrick") +
  scale_y_continuous(sec.axis = sec_axis( ~ ., name = "Temperature (°C)")) +
  x_scale

hum_plot <- ggplot(measurements, aes(x = X1, y = X3)) +
  my_theme +
  geom_line(color = "deepskyblue") +
  scale_y_continuous(sec.axis = sec_axis( ~ ., name = "Humidity (%)")) +
  x_scale

temp_plot / hum_plot

ggsave(commandArgs(trailingOnly = TRUE)[2])
