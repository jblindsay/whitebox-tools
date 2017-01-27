#!/usr/bin/env python
import os
import sys
from subprocess import call

try:
    dir_path = os.path.dirname(os.path.realpath(__file__))
    exe_path = dir_path + "/target/release/"
    os.chdir(exe_path)

    #############
    # Run anova #
    #############
    # cmd = "." + os.path.sep  + "anova"
    # argslist = [
    #     cmd,
    #     '-wd',
    #     '/Users/johnlindsay/Documents/Data/RondeauData/soils/',
    #     '-i',
    #     # 'Local_Mag.dep',
    #     # 'Meso_Mag.dep',
    #     'Broad_Mag.dep',
    #     '-features',
    #     'Soils2.dep',
    #     '-o',
    #     'anova.html',
    #     '-v'
    # ]

    ###############################
    # Run cumulative_distribution #
    ###############################
    # cmd = "." + os.path.sep  + "cumulative_distribution"
    # argslist = [
    #     cmd,
    #     '-wd',
    #     '/Users/johnlindsay/Documents/Data/RondeauData/soils/',
    #     '-i',
    #     'Meso_Mag.dep',
    #     '-features',
    #     'tmp3.dep',
    #     '-o',
    #     'percentiles.html',
    #     '-v'
    # ]

    #########################
    # Run fill_missing_data #
    #########################
    # cmd = "." + os.path.sep + "fill_missing_data"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '/Users/johnlindsay/Documents/data/Rondeau/',
    #     '-i', # input file
    #     # '1km183270487302008GROUPEALTA_filled.dep',
    #     # 'StudyData_filtered2_NN.dep',
    #     'StudyData_filtered_NN.dep',
    #     # 'StudyData_filtered_IDW.dep',
    #     # '418_4688_NN.dep',
    #     '-o', # output file
    #     'StudyData_filtered_filled.dep',
    #     # 'StudyData_Rondeau_IDW_filled.dep',
    #     '-filter=41',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    ####################
    # Run raster_slope #
    ####################
    # cmd = "." + os.path.sep  + "slope"
    # argslist = [
    #     cmd,
    #     '-i',
    #     # '/Users/johnlindsay/Documents/Programming/rust/libgeospatial/testdata/Sample64Bit.tif',
    #     '/Users/johnlindsay/Documents/Programming/rust/libgeospatial/testdata/DEM.tif',
    #     # '/Users/johnlindsay/Documents/Data/Indiana LiDAR/geotiff.tif',
    #     # '/Users/johnlindsay/Documents/Data/Indiana LiDAR/DEM breached.dep',
    #     # '/Users/johnlindsay/Documents/Data/Indiana LiDAR/out.grd',
    #     '-o',
    #     '/Users/johnlindsay/Documents/Data/Indiana LiDAR/out3.dep',
    #     # '/Users/johnlindsay/Documents/Data/Indiana LiDAR/out3.dep',
    #     '-v'
    # ]

    ##############################
    # Run lidar_tophat_transform #
    ##############################
    # cmd = "." + os.path.sep + "lidar_tophat_transform"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '/Users/johnlindsay/Documents/data/Rondeau/',
    #     '-i', # input file
    #     # 'StudyData.las',
    #     # 'StudyData_filtered3.las',
    #     # '1km183270487302008GROUPEALTA.las',
    #     # '420_4687.las',
    #     # '418_4688.las',
    #     "428_4692.las",
    #     '-o', # output file
    #     'tmp1_EAG.las',
    #     '-dist=10.0',
    #     # '-minz=75.0',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    #############################
    # Run lidar_elevation_slice #
    #############################
    # cmd = "." + os.path.sep  + "lidar_elevation_slice"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     # "/Users/johnlindsay/Documents/data/Rondeau/",
    #     # "/Users/johnlindsay/Documents/data/GullyCreek/LiDAR/1_LiDAR_OMAFRA_PointCloud/LAS_tiles_25m/",
    #     "/Users/johnlindsay/Documents/teaching/GEOG3420/W17/Labs/Lab2/NewLab/data/",
    #     '-i', # input file
    #     # 'StudyData_EAG.las',
    #     # "448000_4827000.las",
    #     # "447000_4828000.las",
    #     # "446000_4829000.las",
    #     "1km183270487302008GROUPEALTA.las",
    #     '-o', # output file,
    #     'test_tile4.las',
    #     '-minz', # minimum elevation
    #     '75.0',
    #     '-maxz', # maximum elevaiton
    #     '155.0',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]


    # #####################################
    # # Run lidar_ground_point_separation #
    # #####################################
    # cmd = "." + os.path.sep + "lidar_ground_point_separation"
    # argslist = [
    #     cmd,
    #     "-wd", # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/Rondeau/',
    #     # "/Users/johnlindsay/Documents/data/Rondeau/",
    #     # "/Users/johnlindsay/Documents/teaching/GEOG3420/W17/Labs/Lab2/NewLab/data/",
    #     # "/Users/johnlindsay/Documents/data/JayStateForest/",
    #     "/Users/johnlindsay/Documents/teaching/GEOG3420/W17/Labs/Lab2/NewLab/data/",
    #     '-i', # input file
    #     # "446000_4829000.las",
    #     # 'StudyData.las',
    #     # 'StudyData_Rondeau2.las',
    #     # "points-2.las",
    #     # '1km183270487302008GROUPEALTA.las',
    #     # '420_4687.las',
    #     # '418_4688.las',
    #     # "428_4692.las",
    #     # "test_tile.las",
    #     "test_tile4.las",
    #     "-o", # output file
    #     "out5.las",
    #     "-dist=3.0",
    #     "-slope=40.0", # maximum slope between neighbouring points
    #     "-minzdiff=0.25", # minimum `elevation above ground for an off-terrain object
    #     "-maxzdiff=2.5", # maximum `elevation above ground for an off-terrain object
    #     # "-minz=400.0",
    #     # '-class', # without this flag, the tool removes OTO points but with this flag, it simply reclassifies points
    #     "-v" # verbose mode; progress will be updated to output stream
    # ]


    ##################
    # Run lidar_info #
    ##################
    # cmd = "." + os.path.sep  + "lidar_info"
    # argslist = [
    #     cmd,
    #     '-i', # input file
    #     '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/StudyData.las',
    #     # '/Users/johnlindsay/Documents/data/Rondeau/StudyData_Rondeau.las',
    #     # '/Users/johnlindsay/Documents/data/JayStateForest/points-2.las',
    #     '-vlr' # display VLRs
    # ]

    ##################
    # Run lidar_join #
    ##################
    # cmd = "." + os.path.sep + "lidar_join"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '-i', # input file
    #     '1km183270487302008GROUPEALTA.las, 1km183280487302008GROUPEALTA.las, 1km183270487402008GROUPEALTA.las, 1km183280487402008GROUPEALTA.las',
    #     '-o', # output file
    #     'StudyData.las',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]


    ###################
    # Run lidar_kappa #
    ###################
    # cmd = "." + os.path.sep  + "lidar_kappa"
    # argslist = [
    #     cmd,
    #     '-wd',
    #     '/Users/johnlindsay/Documents/Data/JohnstonGreen/', #'/Users/johnlindsay/Documents/Research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '-i1',
    #     'output1.las',
    #     '-i2',
    #     'out.las', # reference data
    #     '-o',
    #     'kappa1.html',
    # ]

    ########################
    # Run lidar_normal_vec #
    ########################
    # cmd = "." + os.path.sep + "lidar_normal_vec"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     '/Users/johnlindsay/Documents/Data/',
    #     '-i', # input file
    #     'points.las',
    #     '-o', # output file
    #     'normal_vectors.las',
    #     '-num_points', # number of points used to fit planes
    #     '25',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    #####################
    # Run lidar_reclass #
    #####################
    # cmd = "." + os.path.sep + "lidar_reclass"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '/Users/johnlindsay/Documents/data/Rondeau/',
    #     '-i', # input file
    #     'out4.las',
    #     '-reclass_file', # reclassification data
    #     'reclass.csv',
    #     '-o', # output file
    #     'StudyData_filtered2.las',
    #     '-unclassed_value', # class value assigned to unspecified rgb values
    #     '1',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    #############################
    # Run lidar_remove_outliers #
    #############################
    # cmd = "." + os.path.sep + "lidar_remove_outliers"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     '/Users/johnlindsay/Documents/Data/JohnstonGreen/',
    #     '-i', # input file
    #     'JGreenCombined.las',
    #     '-o', # output file
    #     'out1.las',
    #     '-threshold_density', # threhsold in point density for retaining points.
    #     '1.0',
    #     '-num_neighbours', # num of neighbours used in defining the point density in the region surrounding points
    #     '5',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    ##########################
    # Run lidar_segmentation #
    ##########################
    # Note: if this argslist doesn't include a -dist parameter it is running with a variable search
    # distance for the region growing operation that is determined by the -num_points parameter.
    cmd = "." + os.path.sep + "lidar_segmentation"
    argslist = [
        cmd,
        '-wd', # working directory
        # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
        # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/Rondeau/',
        # '/Users/johnlindsay/Documents/data/JayStateForest/',
        # "/Users/johnlindsay/Documents/data/Rondeau/",
        "/Users/johnlindsay/Documents/teaching/GEOG3420/W17/Labs/Lab2/NewLab/data/",
        '-i', # input file
        # 'out3.las',
        # 'points-2.las',
        # "428_4692.las",
        # "out1.las",
        "out5.las",
        '-o', # output file
        'out2.las',
        '-dist=15.0',
        '-max_norm_angle=10.0', # maximum difference in normal vectors allowable for two neighbouring points within a region
        '-maxzdiff=0.15', # maximum difference in elevation used during region growing operation
        '-detrend=25.0',
        '--classify_ground',
        '-v' # verbose mode; progress will be updated to output stream
    ]

    #######################################
    # Run lidar_segmentation_based_filter #
    #######################################
    # Note: if this argslist doesn't include a -dist parameter it is running with a variable search
    # distance for the region growing operation that is determined by the -num_points parameter.

    # cmd = "." + os.path.sep + "lidar_segmentation_based_filter"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/Rondeau/',
    #     '-i', # input file
    #     # '1km183270487302008GROUPEALTA.las',
    #     # '420_4687.las',
    #     '420_4687_slope_filtered.las',
    #     '-o', # output file
    #     '420_4687_segmentation_filtered.las',
    #     '-num_points', # number of points used to fit planes to calculate normal vectors
    #     '50',
    #     '-dist',
    #     '10.0',
    #     '-max_norm_angle', # maximum difference in normal vectors allowable for two neighbouring points within a region
    #     '5.0',
    #     '-maxzdiff', # maximum difference in elevation used during region growing operation
    #     '0.250',
    #     '-oto_threshold', # maximum difference in elevation used to distinguish OTOs from ground segments
    #     '0.2',
    #     #'-class', # without this flag, the tool removes OTO points but with this flag, it simply reclassifies points
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    ################################
    # Run lidar_slope_based_filter #
    ################################
    # cmd = "." + os.path.sep + "lidar_slope_based_filter"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/Rondeau/',
    #     '-i', # input file
    #     '1km183270487302008GROUPEALTA.las',
    #     # '420_4687.las',
    #     # '418_4688.las',
    #     '-o', # output file
    #     # '418_4688_slope_filtered.las',
    #     '1km183270487302008GROUPEALTA_slope_filtered.las',
    #     '-dist', # variable search distance
    #     '7.0',
    #     '-slope', # maximum slope between neighbouring points
    #     '60.0',
    #     '-minzdiff', # minimum elevation above ground for an off-terrain object
    #     '0.2',
    #     # '-class', # without this flag, the tool removes OTO points but with this flag, it simply reclassifies points
    #     '-last_only', # all first and intermediate returns are considered OTOs and filtering occurs on only and last return points
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    ##################
    # Run lidar_tile #
    ##################
    # cmd = "." + os.path.sep + "lidar_tile"
    # argslist = [
    #     cmd,
    #     '-i', # input file
    #     '/Users/johnlindsay/Documents/Data/JohnstonGreen/JGreenCombined.las', #'1km183270487402008GROUPEALTA.las',
    #     '-width_x', # tile width in x dimension
    #     '50.0',
    #     '-width_y', # tile width in y dimension
    #     '50.0',
    #     '-origin_x', # origin x dimension
    #     '0.0',
    #     '-origin_y', # origin y dimension
    #     '0.0',
    #     '-min_points', # minimum number of points in a tile to be output
    #     '100',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    ##################################
    # Run remove_off_terrain_objects #
    ##################################
    # cmd = "." + os.path.sep + "remove_off_terrain_objects"
    # argslist = [
    #     cmd,
    #     '-wd', # working directory
    #     '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
    #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/Rondeau/',
    #     '-i', # input file
    #     # '1km183270487302008GROUPEALTA_filled.dep',
    #     'small DEM.dep',
    #     # '418_4688_NN.dep',
    #     '-o', # output file
    #     'tmp17.dep',
    #     '-filter',
    #     '49',
    #     '-slope',
    #     '15.0',
    #     '-v' # verbose mode; progress will be updated to output stream
    # ]

    retcode = call(argslist, shell=False)
    if retcode < 0:
        print >>sys.stderr, "Child was terminated by signal", -retcode

except OSError as e:
    print >>sys.stderr, "Execution failed:", e
